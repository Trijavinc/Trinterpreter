use core::fmt;
use std::{iter::Peekable, rc::Rc};

use anyhow::{anyhow, Result};

use crate::lexer::{Lexer, Token};

pub struct Parser {
    lexer: Peekable<Lexer>,
}

impl Parser {
    pub fn new(input: String) -> Self {
        return Self {
            lexer: Lexer::new(input).peekable(),
        };
    }

    pub fn parse(&mut self) -> Vec<Result<AST>> {
        return self.parse_statement();
    }

    fn expect_peek(&mut self, tok: Token) -> Result<()> {
        let error = match self.lexer.peek() {
            Some(Ok(next)) => {
                if *next == tok {
                    self.lexer.next();
                    return Ok(());
                }
                Err(anyhow!("expected {tok}"))
            }
            Some(Err(err)) => Err(anyhow!("{:?}", err)),
            None => return Err(anyhow!("expected {tok}")),
        };

        return error;
    }

    fn parse_statement(&mut self) -> Vec<Result<AST>> {
        let mut statements: Vec<Result<AST>> = Vec::new();

        while let Some(tok_result) = self.lexer.peek() {
            let token = match tok_result {
                Ok(Token::Fn) => self.parse_fun(),
                Ok(Token::Return) => self.parse_return(),
                Ok(Token::If) => self.parse_if(),
                Ok(Token::Print) => self.parse_print(),
                Ok(Token::Let) => self.parse_let(),
                Ok(Token::EOF) => break,

                Ok(Token::LParen)
                | Ok(Token::Number(_, _))
                | Ok(Token::String(_))
                | Ok(Token::Ident(_))
                | Ok(Token::Bang)
                | Ok(Token::Minus)
                | Ok(Token::True)
                | Ok(Token::Len)
                | Ok(Token::False) => self.parse_expression_statements(),
                Err(err) => Err(anyhow!("{:?}", err)),
                _ => break,
            };

            statements.push(token);
        }

        return statements;
    }

    fn parse_expression(&mut self, prev_binding: u8) -> Result<AST> {
        let l_side = match self.lexer.next() {
            Some(Ok(tok)) => tok,
            Some(Err(err)) => return Err(err),
            None => return Ok(AST::Type(Type::Nil)),
        };

        let mut to_return = match l_side {
            Token::String(val) => AST::Type(Type::String(val)),
            Token::Number(_, num) => AST::Type(Type::Number(num)),
            Token::True => AST::Type(Type::Bool(true)),
            Token::False => AST::Type(Type::Bool(false)),
            Token::Fn => self.parse_fun()?,
            Token::If => self.parse_if()?,
            Token::LBracket => {
                todo!()
            }

            Token::Ident(ident) => {
                if matches!(self.lexer.peek(), Some(Ok(Token::Assign))) {
                    return Ok(AST::Reassign {
                        ident,
                        value: Box::new(self.parse_expression(0)?),
                    });
                }
                AST::Type(Type::Ident(ident))
            }

            Token::Assign | Token::LParen => {
                let r_side = self.parse_expression(0)?;
                if matches!(l_side, Token::LParen) {
                    self.expect_peek(Token::RParen)?;
                    AST::Expr(Op::Grouped, vec![r_side])
                } else {
                    AST::Expr(Op::Assing, vec![r_side])
                }
            }

            Token::Len => {
                self.expect_peek(Token::LParen)?;
                let right = self.parse_expression(0)?;
                self.expect_peek(Token::RParen)?;

                AST::Expr(Op::Len, vec![AST::Len(Box::new(right))])
            }

            Token::Bang | Token::Minus => {
                let op = match l_side {
                    Token::Bang => Op::Bang,
                    Token::Minus => Op::Minus,
                    _ => return Ok(AST::Type(Type::Nil)),
                };

                let (_, r_binding) = self.prefix_binding_power(op);
                let r_side = self.parse_expression(r_binding)?;
                AST::Expr(op, vec![r_side])
            }

            _ => return Err(anyhow!("expected an expression")),
        };

        loop {
            let tok = match self.lexer.peek() {
                Some(Ok(tok)) => tok,
                Some(Err(err)) => panic!("err: {}", err),
                None => {
                    break;
                }
            };

            let op = match tok {
                Token::Plus => Op::Plus,
                Token::Minus => Op::Minus,
                Token::Star => Op::Star,
                Token::Slash => Op::Slash,
                Token::LParen => Op::Fn,
                Token::AssignEqual => Op::AssignEqual,
                Token::Bang => Op::Bang,
                Token::BangEqual => Op::BangEqual,
                Token::Less => Op::Less,
                Token::LessEqual => Op::LessEqual,
                Token::Greater => Op::Greater,
                Token::GreaterEqual => Op::GreaterEqual,
                Token::And => Op::And,
                Token::Or => Op::Or,
                _ => break,
            };

            if let Some((l_binding, _)) = self.postfix_binding_power(op) {
                if l_binding < prev_binding {
                    break;
                }
                self.expect_peek(Token::LParen)?;
                to_return = AST::Call {
                    calle: Box::new(to_return),
                    args: self.parse_args()?,
                };
                self.expect_peek(Token::RParen)?;

                to_return = AST::Expr(op, vec![to_return]);

                continue;
            }

            if let Some((l_binding, r_binding)) = self.infix_binding_power(op) {
                if l_binding < prev_binding {
                    break;
                }
                self.lexer.next();
                let r_side = self.parse_expression(r_binding)?;
                to_return = AST::Expr(op, vec![to_return, r_side]);

                continue;
            }

            break;
        }

        return Ok(to_return);
    }

    fn postfix_binding_power(&self, op: Op) -> Option<(u8, ())> {
        match op {
            Op::Fn | Op::Len => Some((13, ())),
            _ => None,
        }
    }

    fn prefix_binding_power(&self, op: Op) -> ((), u8) {
        match op {
            Op::Plus | Op::Minus | Op::Bang => ((), 11),
            _ => panic!("bad operation"),
        }
    }

    fn infix_binding_power(&self, op: Op) -> Option<(u8, u8)> {
        let binding_power = match op {
            Op::And | Op::Or => (3, 4),
            Op::BangEqual
            | Op::AssignEqual
            | Op::Less
            | Op::LessEqual
            | Op::Greater
            | Op::GreaterEqual => (5, 6),
            Op::Plus | Op::Minus => (7, 8),
            Op::Star | Op::Slash => (9, 10),
            _ => return None,
        };

        return Some(binding_power);
    }

    fn parse_let(&mut self) -> Result<AST> {
        if matches!(self.lexer.peek(), Some(Ok(Token::Let))) {
            self.lexer.next();
        }

        let ident = match self.lexer.next() {
            Some(Ok(Token::Ident(val))) => val,
            Some(Err(err)) => return Err(err),
            _ => return Err(anyhow!("Expected an indentifier")),
        };

        let value = match self.lexer.peek() {
            Some(Ok(Token::Assign)) => self.parse_expression(0)?,
            Some(Err(err)) => panic!("Error: {}", err),
            _ => AST::Type(Type::Nil),
        };

        self.expect_peek(Token::Semicolon)?;

        return Ok(AST::Let {
            ident,
            value: Box::new(value),
        });
    }

    fn parse_return(&mut self) -> Result<AST> {
        self.lexer.next();
        let value = self.parse_expression(0)?;
        self.expect_peek(Token::Semicolon)?;

        return Ok(AST::Return {
            value: Box::new(value),
        });
    }

    fn parse_if(&mut self) -> Result<AST> {
        if matches!(self.lexer.peek(), Some(Ok(Token::If))) {
            self.lexer.next();
        }
        let condition = self.parse_expression(0)?;

        self.expect_peek(Token::LBrace)?;
        let yes = self.parse_block()?;
        self.expect_peek(Token::RBrace)?;

        let no = match self.lexer.peek() {
            Some(Ok(Token::Else)) => {
                self.lexer.next();
                self.expect_peek(Token::LBrace)?;
                let no = self.parse_block()?;
                self.expect_peek(Token::RBrace)?;
                Some(no)
            }
            _ => None,
        };

        return Ok(AST::If {
            condition: Box::new(condition),
            yes,
            no,
        });
    }

    fn parse_block(&mut self) -> Result<Rc<[AST]>> {
        let mut to_return = Vec::new();
        for result in self.parse_statement() {
            match result {
                Ok(ast) => to_return.push(ast),
                Err(err) => return Err(err),
            };
        }

        return Ok(to_return.into());
    }

    fn parse_fun(&mut self) -> Result<AST> {
        if matches!(self.lexer.peek(), Some(Ok(Token::Fn))) {
            self.lexer.next();
        }
        let name = if let Some(Ok(Token::Ident(val))) = self.lexer.peek() {
            let name = val.clone();
            self.lexer.next();
            Some(name)
        } else {
            None
        };
        self.expect_peek(Token::LParen)?;
        let params = self.parse_params();
        self.expect_peek(Token::RParen)?;

        self.expect_peek(Token::LBrace)?;
        let body = self.parse_block()?;
        self.expect_peek(Token::RBrace)?;

        return Ok(AST::Fn { name, params, body });
    }

    fn parse_params(&mut self) -> Rc<[Rc<str>]> {
        let mut params: Vec<Rc<str>> = Vec::new();

        if !matches!(self.lexer.peek(), Some(Ok(Token::RParen))) {
            match self.lexer.next() {
                Some(Ok(Token::Ident(str))) => params.push(str),
                _ => todo!(), // error
            }
        }

        while matches!(self.lexer.peek(), Some(Ok(Token::Comma))) {
            self.lexer.next(); // comsume comma
            match self.lexer.next() {
                Some(Ok(Token::Ident(str))) => params.push(str),
                _ => todo!(), // error
            }
        }

        return params.into();
    }

    fn parse_args(&mut self) -> Result<Rc<[AST]>> {
        let mut params: Vec<AST> = Vec::new();

        if !matches!(self.lexer.peek(), Some(Ok(Token::RParen))) {
            params.push(self.parse_expression(0)?);
        }

        while matches!(self.lexer.peek(), Some(Ok(Token::Comma))) {
            self.lexer.next(); // comsume comma
            params.push(self.parse_expression(0)?);
        }

        return Ok(params.into());
    }

    fn parse_print(&mut self) -> Result<AST> {
        self.lexer.next();

        match self.lexer.peek() {
            Some(Ok(_)) => {
                let to_return = self.parse_expression(0)?;
                self.expect_peek(Token::Semicolon)?;
                Ok(AST::Print(Box::new(to_return)))
            }
            _ => Err(anyhow!("expected an expression")),
        }
    }

    fn parse_expression_statements(&mut self) -> Result<AST> {
        let expr = self.parse_expression(0)?;
        if matches!(self.lexer.peek(), Some(Ok(Token::Semicolon))) {
            self.lexer.next();
        }
        return Ok(expr);
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Op {
    Plus,
    Minus,
    Star,
    Slash,
    Bang,
    Grouped,
    Assing,
    Fn,
    BangEqual,
    LessEqual,
    GreaterEqual,
    Less,
    Greater,
    AssignEqual,
    Or,
    And,
    Len,
}

impl fmt::Display for Op {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Op::Minus => "-",
                Op::Plus => "+",
                Op::Star => "*",
                Op::Assing => "=",
                Op::BangEqual => "!=",
                Op::AssignEqual => "==",
                Op::LessEqual => "<=",
                Op::GreaterEqual => ">=",
                Op::Less => "<",
                Op::Greater => ">",
                Op::Slash => "/",
                Op::Bang => "!",
                Op::And => "and",
                Op::Or => "or",
                Op::Fn => "call",
                Op::Grouped => "group",
                Op::Len => "len",
            }
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    String(Rc<str>),
    Number(f64),
    Ident(Rc<str>),
    Bool(bool),
    Nil,
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // NOTE: this feels more correct:
            // Type::String(s) => write!(f, "\"{s}\""),
            Type::String(s) => write!(f, "{s}"),
            Type::Number(n) => {
                if *n == n.trunc() {
                    // tests require that integers are printed as N.0
                    write!(f, "{n}.0")
                } else {
                    write!(f, "{n}")
                }
            }
            Type::Nil => write!(f, "nil"),
            Type::Bool(b) => write!(f, "{b:?}"),
            Type::Ident(i) => write!(f, "{i}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum AST {
    Type(Type),

    Expr(Op, Vec<AST>),

    Print(Box<AST>),

    Reassign {
        ident: Rc<str>,
        value: Box<AST>,
    },

    Let {
        ident: Rc<str>,
        value: Box<AST>, // Expr
    },

    Len(Box<AST>),

    Fn {
        name: Option<Rc<str>>,
        params: Rc<[Rc<str>]>,
        body: Rc<[AST]>,
    },

    Call {
        calle: Box<AST>,
        args: Rc<[AST]>,
    },

    Return {
        value: Box<AST>, // Expr
    },

    If {
        condition: Box<AST>,
        yes: Rc<[AST]>,
        no: Option<Rc<[AST]>>,
    },
}

impl fmt::Display for AST {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AST::Type(i) => write!(f, "{}", i),
            AST::Print(i) => write!(f, "{}", i),
            AST::Reassign { ident, value } => write!(f, "(= {ident} {value})"),
            AST::Len(expr) => write!(f, "{expr}"),
            AST::Expr(head, rest) => {
                write!(f, "({}", head)?;
                for s in rest {
                    write!(f, " {s}")?
                }
                write!(f, ")")
            }
            AST::Fn { name, params, body } => {
                if let Some(val) = name {
                    write!(f, "{val}")?;
                } else {
                    write!(f, "(fun")?;
                }

                for p in params.iter() {
                    write!(f, " {p}")?;
                }
                for stmt in body.iter() {
                    write!(f, " {}", stmt)?;
                }
                write!(f, "}}")
            }
            AST::Call { calle, args } => {
                write!(f, "({calle}")?;
                for a in args.iter() {
                    write!(f, " {a}")?
                }
                write!(f, ")")
            }
            AST::If { condition, yes, no } => {
                write!(f, "if {condition} {{")?;
                for stmt in yes.iter() {
                    write!(f, " {stmt}")?;
                }

                write!(f, " }} ")?;

                if let Some(no) = no {
                    write!(f, "else {{")?;
                    for stmt in no.iter() {
                        write!(f, " {stmt}")?;
                    }
                    write!(f, " }}")?;
                }
                write!(f, "")
            }
            AST::Return { value } => {
                write!(f, "return {value}")
            }
            AST::Let { ident, value } => {
                write!(f, "{ident}")?;
                write!(f, "{value}")
            }
        }
    }
}

#[cfg(test)]
mod tests {

    use std::rc::Rc;

    use super::{Op, Parser, Type, AST};
    use anyhow::Result;

    #[test]
    fn expressions_statement() -> Result<()> {
        let test_cases = vec![
            // Input, Expected Output
            ("(-2 - 1) * 3", "(* (group (- (- 2.0) 1.0)) 3.0)"),
            ("3 - (-2)", "(- 3.0 (group (- 2.0)))"),
            (
                "23 + (5 - 3 * 5) / 10;",
                "(+ 23.0 (/ (group (- 5.0 (* 3.0 5.0))) 10.0))",
            ),
            (
                "(10 - 0) * 10 > 4 / 3;",
                "(> (* (group (- 10.0 0.0)) 10.0) (/ 4.0 3.0))",
            ),
            ("!true", "(! true)"),
            ("!!false != true", "(!= (! (! false)) true)"),
            (
                "dos + 3 - mutilple(3) / 100;",
                "(- (+ dos 3.0) (/ (call (mutilple 3.0)) 100.0))",
            ),
        ];

        for (input, expected) in test_cases {
            let asts = Parser::new(input.to_owned()).parse();
            for result in asts {
                assert_eq!(false, matches!(result, Err(_)));
                assert_eq!(expected, format!("{}", result.unwrap()).as_str());
            }
        }

        Ok(())
    }

    #[test]
    fn let_stmt() -> Result<()> {
        let input = r#"let num = 1;
        let num2 = 2;
        let num3 = 3;
        "#
        .to_string();

        let expected = vec![
            AST::Let {
                ident: "num".into(),
                value: Box::new(AST::Expr(Op::Assing, vec![AST::Type(Type::Number(1.0))])),
            },
            AST::Let {
                ident: "num2".into(),
                value: Box::new(AST::Expr(Op::Assing, vec![AST::Type(Type::Number(2.0))])),
            },
            AST::Let {
                ident: "num3".into(),
                value: Box::new(AST::Expr(Op::Assing, vec![AST::Type(Type::Number(3.0))])),
            },
        ];

        let mut parser = Parser::new(input);
        let statements = parser.parse();
        assert_eq!(statements.len(), expected.len());

        for (result, expected_ast) in statements.into_iter().zip(expected.iter()) {
            match result {
                Ok(ast) => assert_eq!(&ast, expected_ast),
                Err(err) => return Err(err),
            }
        }

        Ok(())
    }

    #[test]
    fn print_stmt() -> Result<()> {
        let input = "print 42;";
        let expected = AST::Print(Box::new(AST::Type(Type::Number(42.0))));

        let mut parser = Parser::new(input.to_string());
        let statements = parser.parse();
        assert_eq!(statements.len(), 1);

        match &statements[0] {
            Ok(ast) => assert_eq!(ast, &expected),
            Err(err) => return Err(anyhow::anyhow!("Parsing failed: {}", err)),
        }

        Ok(())
    }

    #[test]
    fn reassign_stmt() -> Result<()> {
        let input = "x = 10;";
        let expected = AST::Reassign {
            ident: "x".into(),
            value: Box::new(AST::Expr(Op::Assing, vec![AST::Type(Type::Number(10.0))])),
        };

        let mut parser = Parser::new(input.to_string());
        let statements = parser.parse();
        assert_eq!(statements.len(), 1);

        match &statements[0] {
            Ok(ast) => assert_eq!(ast, &expected),
            Err(err) => return Err(anyhow::anyhow!("Parsing failed: {}", err)),
        }

        Ok(())
    }

    #[test]
    fn len_expr() -> Result<()> {
        let input = "len(\"hello\");";
        let expected = AST::Expr(
            Op::Len,
            vec![AST::Len(Box::new(AST::Type(Type::String("hello".into()))))],
        );

        let mut parser = Parser::new(input.to_string());
        let statements = parser.parse();
        assert_eq!(statements.len(), 1);

        match &statements[0] {
            Ok(ast) => assert_eq!(ast, &expected),
            Err(err) => return Err(anyhow::anyhow!("Parsing failed: {}", err)),
        }

        Ok(())
    }

    #[test]
    fn fn_stmt() -> Result<()> {
        let input = "fn add(a, b) { return a + b; }";
        let expected = AST::Fn {
            name: Some("add".into()),
            params: Rc::new(["a".into(), "b".into()]),
            body: Rc::new([AST::Return {
                value: Box::new(AST::Expr(
                    Op::Plus,
                    vec![
                        AST::Type(Type::Ident("a".into())),
                        AST::Type(Type::Ident("b".into())),
                    ],
                )),
            }]),
        };

        let mut parser = Parser::new(input.to_string());
        let statements = parser.parse();
        assert_eq!(statements.len(), 1);

        match &statements[0] {
            Ok(ast) => assert_eq!(ast, &expected),
            Err(err) => return Err(anyhow::anyhow!("Parsing failed: {}", err)),
        }

        Ok(())
    }

    #[test]
    fn call_expr() -> Result<()> {
        let input = "add(1, 2);";
        let expected = AST::Expr(
            Op::Fn,
            vec![AST::Call {
                calle: Box::new(AST::Type(Type::Ident("add".into()))),
                args: Rc::new([AST::Type(Type::Number(1.0)), AST::Type(Type::Number(2.0))]),
            }],
        );

        let mut parser = Parser::new(input.to_string());
        let statements = parser.parse();
        assert_eq!(statements.len(), 1);

        match &statements[0] {
            Ok(ast) => assert_eq!(ast, &expected),
            Err(err) => return Err(anyhow::anyhow!("Parsing failed: {}", err)),
        }

        Ok(())
    }

    #[test]
    fn return_stmt() -> Result<()> {
        let input = "return 42;";
        let expected = AST::Return {
            value: Box::new(AST::Type(Type::Number(42.0))),
        };

        let mut parser = Parser::new(input.to_string());
        let statements = parser.parse();
        assert_eq!(statements.len(), 1);

        match &statements[0] {
            Ok(ast) => assert_eq!(ast, &expected),
            Err(err) => return Err(anyhow::anyhow!("Parsing failed: {}", err)),
        }

        Ok(())
    }

    #[test]
    fn if_stmt() -> Result<()> {
        let input = "if x > 0 { print x; } else { print 0; }";
        let expected = AST::If {
            condition: Box::new(AST::Expr(
                Op::Greater,
                vec![
                    AST::Type(Type::Ident("x".into())),
                    AST::Type(Type::Number(0.0)),
                ],
            )),
            yes: Rc::new([AST::Print(Box::new(AST::Type(Type::Ident("x".into()))))]),
            no: Some(Rc::new([AST::Print(Box::new(AST::Type(Type::Number(
                0.0,
            ))))])),
        };

        let mut parser = Parser::new(input.to_string());
        let statements = parser.parse();
        assert_eq!(statements.len(), 1);

        match &statements[0] {
            Ok(ast) => assert_eq!(ast, &expected),
            Err(err) => return Err(anyhow::anyhow!("Parsing failed: {}", err)),
        }

        Ok(())
    }
}
