/*
>* 文章名称：从零编写一个解析器（3）—— 解析 MySQL 建表语句
>* 参考地址：https://github.com/Geal/nom/blob/master/doc/making_a_new_parser_from_scratch.md
>* 文章来自：https://github.com/suhanyujie/my-parser-rs
>* 标签：Rust，parser

根据 MySQL [官方文档](https://dev.mysql.com/doc/refman/8.0/en/create-table.html)，我们可以了解一下 MySQL 创建表的语法。
由于我们的需求比较简单，只针对特定的建表语句，解析出字段结构，并转成 go struct 即可，因此我们不需要实现所有的建表语法。只需实现解析最常规的 sql 语句。

```
CREATE [TEMPORARY] TABLE [IF NOT EXISTS] tbl_name
    (create_definition,...)
    [table_options]
    [partition_options]
```

我们甚至可以简化成：

```
CREATE TABLE tbl_name
    (create_definition,...)
    [table_options]
    [partition_options]
```

也许你对 MySQL 建表语句很熟悉，但可能你对解析 sql 语句不会很熟练，不过没关系，就像你编写大型程序一样，对其进行模块拆分，然后各个击破。

上面的建表语句规则中，我们先从 `create_definition` 开始，它的定义如下：

```
create_definition: {
    col_name column_definition
  | {INDEX | KEY} [index_name] [index_type] (key_part,...)
      [index_option] ...
  | {FULLTEXT | SPATIAL} [INDEX | KEY] [index_name] (key_part,...)
      [index_option] ...
  | [CONSTRAINT [symbol]] PRIMARY KEY
      [index_type] (key_part,...)
      [index_option] ...
  | [CONSTRAINT [symbol]] UNIQUE [INDEX | KEY]
      [index_name] [index_type] (key_part,...)
      [index_option] ...
  | [CONSTRAINT [symbol]] FOREIGN KEY
      [index_name] (col_name,...)
      reference_definition
  | check_constraint_definition
}

column_definition: {
    data_type [NOT NULL | NULL] [DEFAULT {literal | (expr)} ]
      [VISIBLE | INVISIBLE]
      [AUTO_INCREMENT] [UNIQUE [KEY]] [[PRIMARY] KEY]
      [COMMENT 'string']
      [COLLATE collation_name]
      [COLUMN_FORMAT {FIXED | DYNAMIC | DEFAULT}]
      [ENGINE_ATTRIBUTE [=] 'string']
      [SECONDARY_ENGINE_ATTRIBUTE [=] 'string']
      [STORAGE {DISK | MEMORY}]
      [reference_definition]
      [check_constraint_definition]
  | data_type
      [COLLATE collation_name]
      [GENERATED ALWAYS] AS (expr)
      [VIRTUAL | STORED] [NOT NULL | NULL]
      [VISIBLE | INVISIBLE]
      [UNIQUE [KEY]] [[PRIMARY] KEY]
      [COMMENT 'string']
      [reference_definition]
      [check_constraint_definition]
}
```

## 构建字段定义
根据上方的 column_definition 的规则，我们可以逐个实现对应的解析器，但为了简化，并且最快速的实现我们的需求 —— 解析通用建表语句。所谓通用建表语句，指的是创建表的 ddl。

既然只是考虑实现最简单可用的版本，我们挑选一下其中的规则集合：

```
column_definition: {
    data_type [NOT NULL | NULL] [DEFAULT literal ]
    [AUTO_INCREMENT] [UNIQUE [KEY]] [[PRIMARY] KEY]
    [COMMENT 'string']
    [COLLATE collation_name]
}
```

## 参考
* https://dev.mysql.com/doc/refman/8.0/en/create-table.html
* https://github.com/Geal/nom/blob/master/doc/making_a_new_parser_from_scratch.md
* https://github.com/ms705/nom-sql

*/
use crate::parse_string::parse_str_with_escaped_and_combine;
use nom::{
    branch::alt,
    bytes::complete::{tag, tag_no_case, take_while1},
    character::complete::u32 as nom_u32,
    character::complete::{alphanumeric1, char as nom_char},
    character::complete::{space0, space1},
    combinator::opt,
    multi::{fold_many1, many0, many1, many_m_n},
    sequence::{preceded, tuple},
    IResult,
};

/// 解析类型的定义，如：`int not null default '1' comment 'main key'`
/// 在这其中，最重要的信息是 类型、默认值、注释
fn parse_column_definition1(input: &str) -> IResult<&str, String> {
    let mut parser = tuple((
        tag_no_case("int"),
        space1,
        alt((
            tag_no_case("not null default 1"),
            tag_no_case("default null"),
        )),
        space1,
        preceded(tag("comment "), parse_str_with_escaped_and_combine),
    ));
    match parser(input) {
        Ok((remain, (typ, _, _, _, comment_str))) => Ok((remain, comment_str)),
        Err(err) => Err(err),
    }
}

/// 解析 default 部分
fn parse_column_definition_of_default(input: &str) -> IResult<&str, String> {
    match tuple((
        tag_no_case("default"),
        space1,
        parse_str_with_escaped_and_combine,
    ))(input)
    {
        Ok((remain, (_, _, default_val))) => Ok((remain, default_val)),
        Err(err) => Err(err),
    }
}

fn parse_column_definition_of_null(input: &str) -> IResult<&str, String> {
    match alt((tag_no_case("null"), tag_no_case("not null")))(input) {
        Ok((remain, null_val)) => Ok((remain, null_val.to_string())),
        Err(err) => Err(err),
    }
}

// todo
fn parse_column_definition_of_data_type(input: &str) -> IResult<&str, String> {
    match alt((tag_no_case("null"), tag_no_case("not null")))(input) {
        Ok((remain, null_val)) => Ok((remain, null_val.to_string())),
        Err(err) => Err(err),
    }
}

fn identifier_char_parser(input: &str) -> IResult<&str, String> {
    match alt((alphanumeric1, tag("_")))(input) {
        Ok((remain, parse_res)) => Ok((remain, parse_res.to_string())),
        Err(err) => Err(err),
    }
}

fn sql_identifier(input: &str) -> IResult<&str, String> {
    let identifier_parser = fold_many1(identifier_char_parser, String::new, |mut string, tmp| {
        string += &tmp;
        string
    });
    match tuple((
        many_m_n(0, 1, nom_char('`')),
        identifier_parser,
        many_m_n(0, 1, nom_char('`')),
    ))(input)
    {
        Ok((remain, (_, table_name, _))) => Ok((remain, table_name.to_string())),
        Err(err) => Err(err),
    }
}

// 处理类型
#[derive(Debug, PartialEq, Eq)]
enum DataTypeEnum {
    TinyInt,
    SmallInt,
    Int,
    Bigint,
    VarChar(u32),
    DateTime(u32),
    Text,
    BigText,
    Decimal(u8),
    Unknown,
}

fn type_int_size(input: &str) -> IResult<&str, u32> {
    match tuple((tag("("), nom_u32, tag(")")))(input) {
        Ok((remain, (_, int_size, _))) => Ok((remain, int_size)),
        Err(err) => Err(err),
    }
}

fn type_tiny_int(input: &str) -> IResult<&str, DataTypeEnum> {
    match tuple((tag_no_case("tinyint"), opt(type_int_size)))(input) {
        Ok((remain, (_, _))) => Ok((remain, DataTypeEnum::TinyInt)),
        Err(err) => Err(err),
    }
}

fn type_some_int(input: &str) -> IResult<&str, DataTypeEnum> {
    match tuple((
        alt((
            tag_no_case("int"),
            tag_no_case("bigint"),
            tag_no_case("smallint"),
            tag_no_case("tinyint"),
        )),
        opt(type_int_size),
    ))(input)
    {
        Ok((remain, (flag, _))) => {
            let parse_res = match flag {
                "int" => DataTypeEnum::Int,
                "bigint" => DataTypeEnum::Bigint,
                "smallint" => DataTypeEnum::SmallInt,
                "tinyint" => DataTypeEnum::TinyInt,
                _ => DataTypeEnum::Unknown,
            };
            Ok((remain, parse_res))
        }
        Err(err) => Err(err),
    }
}

fn type_collate(input: &str) -> IResult<&str, String> {
    match tuple((space1, tag_no_case("collate"), space1, sql_identifier))(input) {
        Ok((remain, (_, _, _, collate_name))) => Ok((remain, collate_name)),
        Err(err) => Err(err),
    }
}

// `user_name` varchar(50) COLLATE utf8mb4_bin DEFAULT NULL COMMENT '用户名',
fn type_varchar(input: &str) -> IResult<&str, DataTypeEnum> {
    match tuple((tag_no_case("varchar"), type_int_size, opt(type_collate)))(input) {
        // 暂时忽略字符集排序
        Ok((remain, (_, size, _))) => Ok((remain, DataTypeEnum::VarChar(size))),
        Err(err) => Err(err),
    }
}

// datetime(3) DEFAULT NULL COMMENT '创建时间',
fn type_datetime(input: &str) -> IResult<&str, DataTypeEnum> {
    match tuple((tag_no_case("datetime"), type_int_size))(input) {
        Ok((remain, (_, size))) => Ok((remain, DataTypeEnum::DateTime(size))),
        Err(err) => Err(err),
    }
}

fn type_text(input: &str) -> IResult<&str, DataTypeEnum> {
    match tuple((tag_no_case("text"), opt(type_collate)))(input) {
        Ok((remain, (_, _))) => Ok((remain, DataTypeEnum::Text)),
        Err(err) => Err(err),
    }
}

fn type_bigtext(input: &str) -> IResult<&str, DataTypeEnum> {
    match tuple((tag_no_case("bigtext"), opt(type_collate)))(input) {
        Ok((remain, (_, _))) => Ok((remain, DataTypeEnum::BigText)),
        Err(err) => Err(err),
    }
}

// decimal(30)
// 最大可达 65
fn type_decimal(input: &str) -> IResult<&str, DataTypeEnum> {
    match tuple((tag_no_case("decimal"), type_int_size))(input) {
        Ok((remain, (_, size))) => Ok((remain, DataTypeEnum::Decimal(size as u8))),
        Err(err) => Err(err),
    }
}

fn parse_data_type(input: &str) -> IResult<&str, DataTypeEnum> {
    match alt((type_some_int, type_varchar, type_datetime, type_decimal))(input) {
        Ok((remain, parse_res)) => Ok((remain, parse_res)),
        Err(err) => Err(err),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 字符串字面量的声明可以参考 https://doc.rust-lang.org/reference/tokens.html#raw-string-literals
    #[test]
    fn test_parse_sql1() {
        let sql_str1 = r#"CREATE TABLE parse_t(
            id INT NOT NULL,
            cv1 VARCHAR(20) DEFAULT "",
            tt TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            dayweek SMALLINT,
            cv2 INT,
            PRIMARY KEY(id)
        ) ENGINE=innodb DEFAULT CHARSET=utf8mb4;"#;
        let lines: Vec<String> = sql_str1
            .lines()
            .filter(|l| {
                !l.is_empty()
                    && !l.starts_with("#")
                    && !l.starts_with("--")
                    && !(l.starts_with("/*") && l.ends_with("*/;"))
            })
            .map(|l| {
                if !(l.ends_with("\n") || l.ends_with(";")) {
                    String::from(l) + "\n"
                } else {
                    String::from(l)
                }
            })
            .collect();
        //let res = parse_queryset(&lines);

        println!("lines: {:?}", &lines);
    }

    #[test]
    fn test_parse_column_definition1() {
        let input = r##"int not null default 1 comment "主键"
        "##;
        println!("input is: {}", &input);
        let res = parse_column_definition1(input);
        println!("{:?}", res);
    }

    #[test]
    fn test_parse_column_definition_of_default() {
        let input = r##"default "1""##;
        assert_eq!(
            parse_column_definition_of_default(input),
            Ok(("", "1".to_string()))
        );
    }

    #[test]
    fn test_sql_identifier() {
        assert_eq!(
            sql_identifier("work_user"),
            Ok(("", "work_user".to_string()))
        );
        assert_eq!(
            sql_identifier("`work_user`"),
            Ok(("", "work_user".to_string()))
        );
    }

    #[test]
    fn test_type_tiny_int() {
        assert_eq!(
            type_tiny_int("tinyint(10)"),
            Ok(("", DataTypeEnum::TinyInt))
        );
        assert_eq!(
            type_some_int("tinyint(10)"),
            Ok(("", DataTypeEnum::TinyInt))
        );
        assert_eq!(type_some_int("bigint"), Ok(("", DataTypeEnum::Bigint)));
    }

    #[test]
    fn test_type_collate() {
        assert_eq!(
            type_collate(" COLLATE utf8mb4_bin"),
            Ok(("", "utf8mb4_bin".to_string()))
        );
    }

    #[test]
    fn test_parse_data_type() {
        assert_eq!(parse_data_type("int"), Ok(("", DataTypeEnum::Int)));
        assert_eq!(
            parse_data_type("bigint(20)"),
            Ok(("", DataTypeEnum::Bigint))
        );
        assert_eq!(
            parse_data_type("varchar(255)"),
            Ok(("", DataTypeEnum::VarChar(255)))
        );
        assert_eq!(
            parse_data_type("varchar(50) COLLATE utf8mb4_bin"),
            Ok(("", DataTypeEnum::VarChar(50)))
        );
    }
}
