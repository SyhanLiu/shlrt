#![recursion_limit = "1024"]
use shlrt_macros::attr_fn;
use shlrt_macros::{make_test, AnswerFn};
use Vec;

/// item: an item, like a function, struct, module, etc.
/// block: a block (i.e. a block of statements and/or an expression, surrounded by braces)
/// stmt: a statement
/// pat: a pattern
/// expr: an expression
/// ty: a type
/// ident: an identifier
/// path: a path (e.g. foo, ::std::mem::replace, transmute::<_, int>, …)
/// meta: a meta item; the things that go inside #[...] and #![...] attributes
/// tt: a single token tree

macro_rules! my_new_vec {
    ($ele:expr;$n:expr) => {
        vec![$ele; $n]
    };

    ($($elem:expr),*;$vv:expr) => {
        {
            let mut v = Vec::new();
            $(v.push($elem);)*
            v.append(&mut $vv);
            v
        }
    };

    ($($elem:expr),*) => {
        {
            let mut v = Vec::new();
            $(v.push($elem);)*
            v
        }
    };
}

fn test_vec() {
    let mut v = my_new_vec![1, 2; my_new_vec![9,10]];
    let mut s = my_new_vec!(1;10);
    println!("{:?}", s);
    println!("{:?}", v);
}

macro_rules! my_stringify {
    ($e:expr) => {
        stringify!($e)
    };
}

fn test_stringify() {
    println!("{:?}", stringify!(dummy(2 * (1 + (3)))));
    println!("{:?}", my_stringify!(dummy(2 * (1 + (3)))));
}

// 类函数过程宏
make_test!();

#[attr_fn(GTS)]
fn attr_fn() {}

#[derive(AnswerFn)]
struct Ans;

fn main() {
    println!("{}", answer());
    let a = Ans {};
    a.go_to_sleep();
}
