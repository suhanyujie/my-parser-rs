/*!
代码主题：字符串解析
参考地址：https://github.com/Geal/nom/blob/master/examples/string.rs

在代码中，我们经常会声明变量、声明字符串，然后编写业务逻辑，然后你是否有想过，编译器是如何读懂你的变量声明，你的代码逻辑。
在这个文章中，我们先从字符串开始，了解如何通过代码来识别代码，解析你所编写的内容是什么。

在 Rust 中，我们可以声明一个字符串：`let s1 = String::from("hello world");`
也可以像这样声明一个字符引用（字符串常量）：`let s1: &'static = "hello world";`
但 Rust 的字符串中比较特殊，首先，Rust 中的字符类型是 Unicode 标量值，其中可以存储所有的 utf-8 字符，[占用 4 字节](https://doc.rust-lang.org/std/primitive.char.html)。

Rust 的字符串是 utf-8 编码，长度可动态增长的类型，它在底层通常是由一些列的字节序列构成，经过一些特定的编码后，就能得到你想要的字符串了。

此外，在 Rust 字符串中，还支持携带转义的 Unicode 字符，如：`String::from("\u{1231}")`


*/

fn demo1() {
    println!("{}", String::from("\u{1231}")); // ሱ
    println!("{}", 'c' as u32);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test1() {
        demo1();
    }
}
