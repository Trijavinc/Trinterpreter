# Monkey Language Interpreter

This project is a Rust implementation of the interpreter for the **monkelang** programming language, originally designed by **Thorsten Ball**. The interpreter includes a command-line interface (CLI) with multiple functionalities.

![App Screenshot](./.github/app_image.png)

## Language features
- Variable bindings
- Integers and booleans
- A string data structure
- An array data structure
- Arithmetic expressions
- Built-in functions
- First-class and higher-order functions • closures

## Usage 
Make sure you compile the rust code:
```
cargo build --realease
```

### Commands

**Tokenization**
```
monkelang tokenize <file>
```
**Parsing**
The parsing will print out 
```
monkelang parse <file>
```
**Evaluation**
```
monkelang eval <file>
```
**REPL (Read-Eval-Print Loop)**
```
monkelang repl
```
## Examples

### Variable bindings

Once the interpreter is running, you can start writing Monkey Language code directly in the interactive console. Some usage examples:
```
>> let x = 5;
>> let y = 10;
>> x + y
15

>> let add = fn(a, b) { a + b };
>> add(5, 5)
10

>> let arr = [1, 2, 3, 4, 5];
>> arr[2]
3
```

### Control flow structures

Monkey Language supports `if` and `else` statements:
```
if x > y {
    return x;
} else {
    return y;
}

let isNil = if nil {
    true
} else {
    false
};
```

### Higher-order functions

You can pass functions as arguments and return them as results:
```
let twice = fn(f, x) {
    return f(f(x));
};

fn addTwo(x) {
    return x + 2;
}

twice(addTwo, 2); // Retorna 6
```

### Built-in Methods
Monkey Language includes some built-in methods for common data types:

```
>> len("Hello")
5

>> first([1, 2, 3])
1

>> last([1, 2, 3])
3

>> rest("hello")
ello

>> print [23, 69, 31];
[23, 69, 31]

>> let arr = [1, 2];
>> push(arr, 3);
>> arr
[1, 2, 3]
```
