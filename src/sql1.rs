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
* 字符串字面量的声明可以参考 https://doc.rust-lang.org/reference/tokens.html#raw-string-literals

*/

//! 使用 nom 解析 MySQL 的建表语句
use crate::parse_string::{
    parse_str_with_escaped_and_combine, parse_str_with_escaped_and_combine_in_single_quote,
};
use nom::{
    branch::alt,
    bytes::complete::{tag, tag_no_case, take_while1},
    character::complete::u32 as nom_u32,
    character::complete::{alphanumeric1, char as nom_char, multispace0},
    character::complete::{space0, space1},
    combinator::opt,
    multi::{fold_many1, many0, many1, many_m_n},
    sequence::{preceded, tuple},
    IResult,
};
use serde::{Deserialize, Serialize};

/// 解析 default 部分
// not null default 1
// not null default '1231231'
// [dataType] default null
// [dataType] default AUTO_INCREMENT
pub fn parse_column_definition_of_default(input: &str) -> IResult<&str, DefaultEnum> {
    match tuple((opt(parse_column_definition_of_not_null), opt(parse_default)))(input) {
        Ok((remain, (_not_null, default_val))) => {
            // 有可能没有 not null 而只有 default ''
            if default_val.is_some() {
                Ok((remain, DefaultEnum::DefaultNull))
            } else {
                Ok((remain, DefaultEnum::DefaultNone))
            }
        }
        Err(err) => Err(err),
    }
}

pub fn parse_column_definition_of_null(input: &str) -> IResult<&str, String> {
    match tag_no_case("null")(input) {
        Ok((remain, null_val)) => Ok((remain, null_val.to_string())),
        Err(err) => Err(err),
    }
}

// 解析 not null
pub fn parse_column_definition_of_not_null(input: &str) -> IResult<&str, String> {
    match tuple((space1, tag_no_case("not"), space1, tag_no_case("null")))(input) {
        Ok((remain, _some_val)) => Ok((remain, "".to_string())),
        Err(err) => Err(err),
    }
}

pub fn identifier_char_parser(input: &str) -> IResult<&str, String> {
    match alt((alphanumeric1, tag("_")))(input) {
        Ok((remain, parse_res)) => Ok((remain, parse_res.to_string())),
        Err(err) => Err(err),
    }
}

/// 解析 sql 中的标识符，包括字段名、表名等
pub fn sql_identifier(input: &str) -> IResult<&str, String> {
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
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DataTypeEnum {
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

#[derive(Debug, PartialEq, Eq)]
pub enum DefaultEnum {
    DefaultNone, // 没有 default 语句
    DefaultNull,
    DefaultInt(u32),
    DefaultStr(String),
    DefaultCurStamp,
    DefaultAutoIncrement,
    DefaultCurStampOnUpdateCurStamp,
    DefaultOnUpdateCurStamp,
    Unknown,
}

pub fn type_int_size(input: &str) -> IResult<&str, u32> {
    match tuple((tag("("), nom_u32, tag(")")))(input) {
        Ok((remain, (_, int_size, _))) => Ok((remain, int_size)),
        Err(err) => Err(err),
    }
}

pub fn type_tiny_int(input: &str) -> IResult<&str, DataTypeEnum> {
    match tuple((tag_no_case("tinyint"), opt(type_int_size)))(input) {
        Ok((remain, (_, _))) => Ok((remain, DataTypeEnum::TinyInt)),
        Err(err) => Err(err),
    }
}

pub fn parse_int_auto_increment(input: &str) -> IResult<&str, DefaultEnum> {
    match tag_no_case("AUTO_INCREMENT")(input) {
        Ok((remain, _)) => Ok((remain, DefaultEnum::DefaultAutoIncrement)),
        Err(err) => Err(err),
    }
}

pub fn parse_int_is_unsigned(input: &str) -> IResult<&str, Option<i8>> {
    match tuple((space1, tag_no_case("unsigned")))(input) {
        Ok((remain, (_, _))) => Ok((remain, Some(1))),
        Err(err) => Err(err),
    }
}

pub fn type_some_int(input: &str) -> IResult<&str, DataTypeEnum> {
    match tuple((
        alt((
            tag_no_case("int"),
            tag_no_case("bigint"),
            tag_no_case("smallint"),
            tag_no_case("tinyint"),
        )),
        opt(type_int_size),
        opt(parse_int_is_unsigned),
    ))(input)
    {
        Ok((remain, (flag, _, _))) => {
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

pub fn type_collate(input: &str) -> IResult<&str, String> {
    match tuple((space1, tag_no_case("collate"), space1, sql_identifier))(input) {
        Ok((remain, (_, _, _, collate_name))) => Ok((remain, collate_name)),
        Err(err) => Err(err),
    }
}

// `user_name` varchar(50) COLLATE utf8mb4_bin DEFAULT NULL COMMENT '用户名',
pub fn type_varchar(input: &str) -> IResult<&str, DataTypeEnum> {
    match tuple((tag_no_case("varchar"), type_int_size, opt(type_collate)))(input) {
        // 暂时忽略字符集排序
        Ok((remain, (_, size, _))) => Ok((remain, DataTypeEnum::VarChar(size))),
        Err(err) => Err(err),
    }
}

// datetime(3) DEFAULT NULL COMMENT '创建时间',
pub fn type_datetime(input: &str) -> IResult<&str, DataTypeEnum> {
    match tuple((tag_no_case("datetime"), opt(type_int_size)))(input) {
        Ok((remain, (_, size_info))) => {
            if size_info.is_some() {
                Ok((remain, DataTypeEnum::DateTime(size_info.unwrap())))
            } else {
                Ok((remain, DataTypeEnum::DateTime(0)))
            }
        }
        Err(err) => Err(err),
    }
}

pub fn type_text(input: &str) -> IResult<&str, DataTypeEnum> {
    match tuple((tag_no_case("text"), opt(type_collate)))(input) {
        Ok((remain, (_, _))) => Ok((remain, DataTypeEnum::Text)),
        Err(err) => Err(err),
    }
}

pub fn type_bigtext(input: &str) -> IResult<&str, DataTypeEnum> {
    match tuple((tag_no_case("bigtext"), opt(type_collate)))(input) {
        Ok((remain, (_, _))) => Ok((remain, DataTypeEnum::BigText)),
        Err(err) => Err(err),
    }
}

// decimal(30)
// 最大可达 65
pub fn type_decimal(input: &str) -> IResult<&str, DataTypeEnum> {
    match tuple((tag_no_case("decimal"), type_int_size))(input) {
        Ok((remain, (_, size))) => Ok((remain, DataTypeEnum::Decimal(size as u8))),
        Err(err) => Err(err),
    }
}

pub fn parse_data_type(input: &str) -> IResult<&str, DataTypeEnum> {
    match alt((type_some_int, type_varchar, type_datetime, type_decimal))(input) {
        Ok((remain, parse_res)) => Ok((remain, parse_res)),
        Err(err) => Err(err),
    }
}

pub fn parse_default_int(input: &str) -> IResult<&str, DefaultEnum> {
    match tuple((tag_no_case("default"), space1, nom_u32))(input) {
        Ok((remain, (_, _, u32_val))) => Ok((remain, DefaultEnum::DefaultInt(u32_val))),
        Err(err) => Err(err),
    }
}

pub fn parse_default_str(input: &str) -> IResult<&str, DefaultEnum> {
    match tuple((
        tag_no_case("default"),
        space1,
        parse_str_with_escaped_and_combine_in_single_quote,
    ))(input)
    {
        Ok((remain, (_, _, str_val))) => Ok((remain, DefaultEnum::DefaultStr(str_val))),
        Err(err) => Err(err),
    }
}

pub fn parse_comment(input: &str) -> IResult<&str, String> {
    match tuple((
        space1,
        tag_no_case("comment"),
        space1,
        parse_str_with_escaped_and_combine_in_single_quote,
    ))(input)
    {
        Ok((remain, (_, _, _, str_val))) => Ok((remain, str_val.to_string())),
        Err(err) => Err(err),
    }
}

pub fn parse_default_null(input: &str) -> IResult<&str, DefaultEnum> {
    match tuple((tag_no_case("default"), space1, tag_no_case("null")))(input) {
        Ok((remain, (_, _, _))) => Ok((remain, DefaultEnum::DefaultNull)),
        Err(err) => Err(err),
    }
}

pub fn parse_default_on_current_timestamp(input: &str) -> IResult<&str, DefaultEnum> {
    match tuple((
        space1,
        tag_no_case("on"),
        space1,
        tag_no_case("update"),
        space1,
        tag_no_case("current_timestamp"),
    ))(input)
    {
        Ok((remain, (_, _, _, _, _, _))) => Ok((remain, DefaultEnum::DefaultOnUpdateCurStamp)),
        Err(err) => Err(err),
    }
}

pub fn parse_default_current_timestamp(input: &str) -> IResult<&str, DefaultEnum> {
    match tuple((
        tag_no_case("default"),
        space1,
        tag_no_case("CURRENT_TIMESTAMP"),
        opt(parse_default_on_current_timestamp),
    ))(input)
    {
        Ok((remain, (_, _, _, some_val))) => {
            if some_val.is_some() {
                Ok((remain, DefaultEnum::DefaultCurStampOnUpdateCurStamp))
            } else {
                Ok((remain, DefaultEnum::DefaultCurStamp))
            }
        }
        Err(err) => Err(err),
    }
}

// not null default 1
// not null default '1231231'
//  default null
// DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP
// AUTO_INCREMENT 这个也可视为一种默认值
pub fn parse_default(input: &str) -> IResult<&str, DefaultEnum> {
    match tuple((
        space1,
        alt((
            parse_default_int,
            parse_default_str,
            parse_default_null,
            parse_default_current_timestamp,
            parse_int_auto_increment,
        )),
    ))(input)
    {
        Ok((remain, (_, parse_res))) => Ok((remain, parse_res)),
        Err(err) => Err(err),
    }
}

/// MySQL 表中的一个字段
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OneColumn {
    pub name: String,
    pub typ: DataTypeEnum,
    pub comment: String,
}

impl OneColumn {
    fn new(name: String, typ: DataTypeEnum, comment: String) -> Self {
        OneColumn { name, typ, comment }
    }
}

pub fn parse_end_has_comma(input: &str) -> IResult<&str, Option<i8>> {
    let mut parse_has_comma = tuple((tag(","), opt(space0)));
    match parse_has_comma(input) {
        Ok((remain, (_, _))) => Ok((remain, Some(1))),
        Err(err) => Err(err),
    }
}
pub fn parse_end_no_comma(input: &str) -> IResult<&str, Option<i8>> {
    let mut parse_no_comma = tuple((opt(space0), tag("}")));
    match parse_no_comma(input) {
        Ok((remain, (_, _))) => Ok((remain, Some(0))),
        Err(err) => Err(err),
    }
}

// 最后一行字段定义，可能没有逗号
pub fn parse_end_comma(input: &str) -> IResult<&str, Option<i8>> {
    alt((parse_end_has_comma, parse_end_no_comma))(input)
}

/// 解析类型的定义，如：`int not null default '1' comment 'main key'`
/// 在这其中，最重要的信息是 类型、默认值、注释
pub fn parse_column_definition1(input: &str) -> IResult<&str, OneColumn> {
    let mut parser = tuple((
        multispace0,
        sql_identifier,
        space1,
        parse_data_type,
        parse_column_definition_of_default,
        opt(parse_comment),
        space0,
        tag(","),
        opt(multispace0),
    ));
    match parser(input) {
        Ok((remain, (_, column_name, _, column_type, _, opt_comment, _, _, _))) => {
            let mut comment = String::new();
            if opt_comment.is_some() {
                comment = opt_comment.unwrap();
            }
            Ok((remain, OneColumn::new(column_name, column_type, comment)))
        }
        Err(err) => Err(err),
    }
}

pub fn parse_column_definition2(input: &str) -> IResult<&str, OneLineEnum> {
    let mut parser = tuple((
        sql_identifier,
        space1,
        parse_data_type,
        parse_column_definition_of_default,
        opt(parse_comment),
        space0,
        tag(","),
        opt(multispace0),
    ));
    match parser(input) {
        Ok((remain, (column_name, _, column_type, _, opt_comment, _, _, _))) => {
            let mut comment = String::new();
            if opt_comment.is_some() {
                comment = opt_comment.unwrap();
            }
            let one_column = OneColumn::new(column_name, column_type, comment);
            Ok((remain, OneLineEnum::Column(one_column)))
        }
        Err(err) => Err(err),
    }
}

pub fn parse_many_column_definition(input: &str) -> IResult<&str, Vec<OneColumn>> {
    let mut column_define_builder =
        fold_many1(parse_column_definition1, Vec::new, |mut arr, one_column| {
            arr.push(one_column);
            arr
        });

    match tuple((tag("("), space0, column_define_builder, space0, tag(")")))(input) {
        Ok((remain, (_, _, column_arr, _, _))) => Ok((remain, column_arr)),
        Err(err) => Err(err),
    }
}

/// MySQL 表中的索引
#[derive(Debug, PartialEq, Eq)]
pub struct OneIndex {
    name: String,
    using_type: Option<String>, // b-tree、hash、None
    typ: IndexIdxTyeEnum,       //unique key、primary key
    column_names: Vec<String>,
}

/// 一行建表语句的描述。包含字段描述、索引描述等。
#[derive(Debug, PartialEq, Eq)]
pub enum OneLineEnum {
    Column(OneColumn),
    Index(OneIndex),
}

/// 索引类型，如：主键、唯一索引等
#[derive(Debug, PartialEq, Eq)]
pub enum IndexIdxTyeEnum {
    Primary,
    Unique,
    Normal,
    None,
}

/// 解析索引语句中的字段部分 —— (`name`)
pub fn parse_idx_column_name(input: &str) -> IResult<&str, Vec<String>> {
    let column_plus = tuple((sql_identifier, space0, opt(tag(","))));
    let mut parser = tuple((space0, tag("("), many1(column_plus), tag(")")));
    match parser(input) {
        Ok((remain, (_, _, column_name_arr, _))) => {
            let mut name_arr: Vec<String> = vec![];
            for (name, _, _) in column_name_arr {
                name_arr.push(name);
            }
            Ok((remain, name_arr))
        }
        Err(err) => Err(err),
    }
}

/// 解析 using btree 段
pub fn parse_idx_using_struct(input: &str) -> IResult<&str, String> {
    let tree_type = alt((tag_no_case("btree"), tag_no_case("hash")));
    let mut parser = tuple((space1, tag_no_case("using"), space1, tree_type));
    match parser(input) {
        Ok((remain, (_, _, _, tree_type))) => Ok((remain, tree_type.to_string())),
        Err(err) => Err(err),
    }
}

pub fn parse_idx_line(input: &str) -> IResult<&str, OneIndex> {
    let mut parse_index_key = tuple((
        alt((tag_no_case("PRIMARY"), tag_no_case("UNIQUE"))),
        space1,
        tag_no_case("KEY"),
        space1,
        opt(sql_identifier),
        parse_idx_column_name,
        opt(parse_idx_using_struct),
        opt(tag(",")),
        opt(multispace0),
    ));

    match parse_index_key(input) {
        Ok((remain, (typ, _, _, _, idx_name_op, column_name_arr, using_type, _, _))) => {
            let mut idx_name = String::new();
            if idx_name_op.is_some() {
                idx_name = idx_name_op.unwrap();
            }
            let mut typ_enum = IndexIdxTyeEnum::None;
            match typ.to_lowercase().as_str() {
                "primary" => typ_enum = IndexIdxTyeEnum::Primary,
                "unique" => typ_enum = IndexIdxTyeEnum::Unique,
                _ => typ_enum = IndexIdxTyeEnum::None,
            }
            let idx = OneIndex {
                name: idx_name,
                using_type,
                typ: typ_enum,
                column_names: column_name_arr,
            };
            Ok((remain, idx))
        }
        Err(err) => Err(err),
    }
}

// 解析索引声明的前半部分，如：`primary key`、`key`、`unique key`
fn parse_idx_part_prev(input: &str) -> IResult<&str, String> {
    // 解析索引声明的前半部分，如：`primary key`、`key`、`unique key`
    let mut pri_or_uni_idx = tuple((alt((tag_no_case("primary"), tag_no_case("UNIQUE"))), space1));
    match pri_or_uni_idx(input) {
        Ok((remain, (idx_typ, _))) => Ok((remain, idx_typ.to_string())),
        Err(err) => Err(err),
    }
}

/// 解析一行索引声明。如：PRIMARY KEY (`id`)
pub fn parse_idx_line2(input: &str) -> IResult<&str, OneLineEnum> {
    let mut parse_index_key = tuple((
        multispace0,
        opt(parse_idx_part_prev),
        tag_no_case("KEY"),
        space1,
        opt(sql_identifier),
        parse_idx_column_name,
        opt(parse_idx_using_struct),
        opt(tag(",")),
        opt(multispace0),
    ));

    match parse_index_key(input) {
        Ok((remain, (_, typ_op, _, _, idx_name_op, column_name_arr, using_type, _, _))) => {
            let mut typ = "".to_string();
            if typ_op.is_some() {
                typ = typ_op.unwrap();
            }
            let mut idx_name = String::new();
            if idx_name_op.is_some() {
                idx_name = idx_name_op.unwrap();
            }
            let mut typ_enum = IndexIdxTyeEnum::None;
            match typ.to_lowercase().as_str() {
                "primary" => typ_enum = IndexIdxTyeEnum::Primary,
                "unique" => typ_enum = IndexIdxTyeEnum::Unique,
                "" => typ_enum = IndexIdxTyeEnum::Normal,
                _ => typ_enum = IndexIdxTyeEnum::None,
            }
            let idx = OneIndex {
                name: idx_name,
                using_type,
                typ: typ_enum,
                column_names: column_name_arr,
            };
            Ok((remain, OneLineEnum::Index(idx)))
        }
        Err(err) => Err(err),
    }
}

/// 解析建表语句，返回表名
/// 如：CREATE TABLE `demo_table_user`，则返回：`demo_table_user`
pub fn parse_create_table(input: &str) -> IResult<&str, String> {
    let mut parse_if_not_exist = tuple((
        space1,
        tag_no_case("if"),
        space1,
        tag_no_case("not"),
        space1,
        tag_no_case("exists"),
    ));
    let mut parse_create = tuple((
        tag_no_case("create"),
        space1,
        tag_no_case("table"),
        opt(parse_if_not_exist),
        space1,
        sql_identifier,
        multispace0,
    ));
    match parse_create(input) {
        Ok((remain, (_, _, _, _, _, table_name, _))) => Ok((remain, table_name)),
        Err(err) => Err(err),
    }
}

/// 解析建表语句体中的一段，无论是字段声明还是索引声明
pub fn parse_one_define_line(input: &str) -> IResult<&str, OneLineEnum> {
    let mut parser = alt((parse_idx_line2, parse_column_definition2));
    parser(input)
}

/// 匹配多行声明
// pub fn parse_many_column_definition1(input: &str) -> IResult<&str, Vec<OneColumn>> {
//     let mut column_define_builder =
//         fold_many1(parse_one_define_line, Vec::new, |mut arr, one_column| {
//             arr.push(one_column);
//             arr
//         });

//     match tuple((tag("("), space0, column_define_builder, space0, tag(")")))(input) {
//         Ok((remain, (_, _, column_arr, _, _))) => Ok((remain, column_arr)),
//         Err(err) => Err(err),
//     }
// }

/// 解析整个建表语句体中的内容
pub fn parse_many1_define_line(input: &str) -> IResult<&str, Vec<OneLineEnum>> {
    match tuple((
        tag("("),
        multispace0,
        many1(parse_one_define_line),
        multispace0,
        tag(")"),
    ))(input)
    {
        Ok((remain, (_, _, parse_res, _, _))) => Ok((remain, parse_res)),
        Err(err) => Err(err),
    }
}

/// 表配置
#[derive(Debug, PartialEq, Eq)]
pub struct TableOption {
    pub engine: String,
    pub charset: String,
    pub collate: String,
    pub comment: String,
}

/// 解析建表语句中的表配置部分
pub fn parse_table_option(input: &str) -> IResult<&str, TableOption> {
    let mut parser = tuple((
        table_option_engine,
        table_option_char_set,
        table_option_collate,
        table_option_comment,
    ));
    match parser(input) {
        Ok((remain, (engine, charset, collate, comment))) => {
            let table_option = TableOption {
                engine,
                charset,
                collate,
                comment,
            };
            Ok((remain, table_option))
        }
        Err(err) => Err(err),
    }
}

/// 表配置解析 —— ENGINE [=] engine_name
pub fn table_option_engine(input: &str) -> IResult<&str, String> {
    let mut parser = tuple((
        space0,
        tag_no_case("engine"),
        opt(tuple((space0, tag("=")))),
        space0,
        sql_identifier,
    ));
    match parser(input) {
        Ok((remain, (_, _, _, _, engine_name))) => Ok((remain, engine_name)),
        Err(err) => Err(err),
    }
}

/// 表配置解析 —— [DEFAULT] CHARACTER SET [=] charset_name
pub fn table_option_char_set(input: &str) -> IResult<&str, String> {
    let mut parser = tuple((
        opt(tuple((space1, tag_no_case("default")))),
        opt(space0),
        alt((
            tuple((space0, space0, tag_no_case("CHARSET"))),
            tuple((tag_no_case("CHARACTER"), space1, tag_no_case("set"))),
        )),
        tuple((space0, alt((tag("="), space0)), space0)),
        sql_identifier,
    ));
    match parser(input) {
        Ok((remain, (_, _, _, _, char_set_name))) => Ok((remain, char_set_name)),
        Err(err) => Err(err),
    }
}

/// 表配置解析 —— 字符集排序，如 `COLLATE=utf8mb4_bin`
pub fn table_option_collate(input: &str) -> IResult<&str, String> {
    let mut parser = tuple((
        opt(tuple((space1, tag_no_case("default")))),
        opt(space0),
        tag_no_case("COLLATE"),
        tuple((space0, alt((tag("="), space0)), space0)),
        sql_identifier,
    ));
    match parser(input) {
        Ok((remain, (_, _, _, _, collate_name))) => Ok((remain, collate_name)),
        Err(err) => Err(err),
    }
}

/// 表配置解析 —— 表注释、说明
pub fn table_option_comment(input: &str) -> IResult<&str, String> {
    let mut parser = tuple((
        space1,
        tag_no_case("comment"),
        tuple((space0, alt((tag("="), space0)), space0)),
        parse_str_with_escaped_and_combine_in_single_quote,
    ));
    match parser(input) {
        Ok((remain, (_, _, _, comment))) => Ok((remain, comment)),
        Err(err) => Err(err),
    }
}

/// 一个表，表所包含的主要信息
#[derive(Debug, PartialEq, Eq)]
struct TableSchema {
    table_name: String,
    column_arr: Vec<OneColumn>,
    option: TableOption,
}

/// 解析整个建表语句 totest
fn parse_create_sql(input: &str) -> IResult<&str, TableSchema> {
    let mut parser = tuple((
        parse_create_table,
        space0,
        parse_many1_define_line,
        space0,
        parse_table_option,
    ));
    match parser(input) {
        Ok((remain, (table_name, _, column_arr, _, table_option))) => {
            // 拿出字段部分，和索引部分，但索引的信息暂时用不上，所以就不取了
            let column_arr: Vec<OneColumn> = column_arr
                .into_iter()
                .filter(|one_line| match one_line {
                    OneLineEnum::Column(_) => true,
                    _ => false,
                })
                .map(|one_line| match one_line {
                    OneLineEnum::Column(column_line) => column_line,
                    _ => OneColumn {
                        name: "".to_string(),
                        typ: DataTypeEnum::Unknown,
                        comment: "".to_string(),
                    },
                })
                .collect();

            Ok((
                remain,
                TableSchema {
                    table_name,
                    column_arr,
                    option: table_option,
                },
            ))
        }
        Err(err) => Err(err),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_column_definition_of_not_null() {
        assert_eq!(
            parse_column_definition_of_not_null(r##" not null"##),
            Ok(("", "".to_string()))
        );
    }

    #[test]
    fn test_parse_str_with_escaped_and_combine_in_single_quote() {
        let input = "\'hello\'";
        println!("{}", &input);
        assert_eq!(
            parse_str_with_escaped_and_combine_in_single_quote(input),
            Ok(("", "hello".to_string()))
        )
    }

    #[test]
    fn test_parse_default() {
        assert_eq!(
            parse_default(r##" default 1"##),
            Ok(("", DefaultEnum::DefaultInt(1)))
        );
        assert_eq!(
            parse_default(r##" default '1'"##),
            Ok(("", DefaultEnum::DefaultStr("1".to_string())))
        );
    }

    #[test]
    fn test_parse_column_definition_of_default() {
        assert_eq!(
            parse_column_definition_of_default(r##" not null default '1'"##),
            Ok(("", DefaultEnum::DefaultStr("1".to_string())))
        );
        assert_eq!(
            parse_column_definition_of_default(r##" not null default 2"##),
            Ok(("", DefaultEnum::DefaultInt(2)))
        );
        assert_eq!(
            parse_column_definition_of_default(
                r##" NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP"##
            ),
            Ok(("", DefaultEnum::DefaultCurStampOnUpdateCurStamp))
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

    #[test]
    fn test_parse_column_definition1() {
        assert_eq!(
            parse_column_definition1(r##"id int not null default 1 comment "主键","##),
            Ok((
                "",
                OneColumn::new("id".to_string(), DataTypeEnum::Int, "主键".to_string())
            ))
        )
    }

    #[test]
    fn test_parse_many_column_definition() {
        let input = r##"{`pwd` varchar(128) COLLATE utf8mb4_bin NOT NULL DEFAULT '1' COMMENT "加密后的密码",}"##;
        let res = parse_many_column_definition(input);
        println!("{:?}", res);
        assert!(res.is_ok());
        assert!(parse_many_column_definition(
            r##"{`id` bigint unsigned NOT NULL AUTO_INCREMENT COMMENT "主键",
            `id2` bigint unsigned NOT NULL AUTO_INCREMENT COMMENT "主键",}"##
        )
        .is_ok());
    }

    #[test]
    fn test_parse_idx_line2() {
        let input = r##"UNIQUE KEY `user_name_idx` (`user_name`) USING BTREE"##;
        assert_eq!(
            parse_idx_line2(input),
            Ok((
                "",
                OneLineEnum::Index(OneIndex {
                    name: String::from("user_name_idx"),
                    using_type: Some("BTREE".to_string()),
                    typ: IndexIdxTyeEnum::Unique,
                    column_names: vec!["user_name".to_string()],
                })
            ))
        );
    }

    #[test]
    fn test_parse_many_column_definition2() {
        let input = r##"(`id` bigint unsigned NOT NULL AUTO_INCREMENT COMMENT '主键',
  `user_name` varchar(120) COLLATE utf8mb4_bin NOT NULL COMMENT '用户名，全局唯一',
  `nick_name` varchar(200) COLLATE utf8mb4_bin NOT NULL COMMENT '用户昵称',
  `status` smallint NOT NULL DEFAULT '1' COMMENT '用户状态',
  `pwd` varchar(128) COLLATE utf8mb4_bin NOT NULL DEFAULT '' COMMENT '加密后的密码',
  `token` varchar(50) COLLATE utf8mb4_bin NOT NULL DEFAULT '' COMMENT '加密的 token',
  `avatar` varchar(200) COLLATE utf8mb4_bin NOT NULL DEFAULT '' COMMENT '头像地址',
  `sex` varchar(10) COLLATE utf8mb4_bin NOT NULL DEFAULT '' COMMENT '性别',
  `create_time` datetime NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP COMMENT '创建时间',
  `update_time` datetime NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP COMMENT '更新时间',
  `delete_time` datetime DEFAULT NULL COMMENT '删除时间',
    )"##;
        let res = parse_many_column_definition(input);
        println!("{:?}", res);
        assert!(res.is_ok());
    }

    #[test]
    fn test_parse_many1_define_line() {
        let input = r##"(
`id` bigint unsigned NOT NULL AUTO_INCREMENT COMMENT '主键',
PRIMARY KEY (`id`)
        )"##;
        let result: Vec<OneLineEnum> = vec![OneLineEnum::Column(OneColumn {
            name: "id".to_string(),
            typ: DataTypeEnum::Bigint,
            comment: "主键".to_string(),
        })];
        assert_eq!(parse_many1_define_line(input), Ok(("", result)));
    }

    #[test]
    fn test_table_option_engine() {
        //  ENGINE [=] engine_name
        assert_eq!(
            table_option_engine(r##" ENGINE=innodb"##),
            Ok(("", "innodb".to_string()))
        );
        assert_eq!(
            table_option_engine(r##"ENGINE innodb"##),
            Ok(("", "innodb".to_string()))
        );
    }

    #[test]
    fn test_table_option_char_set() {
        assert_eq!(
            table_option_char_set(r##" DEFAULT CHARSET=utf8mb4"##),
            Ok(("", "utf8mb4".to_string()))
        );
        assert_eq!(
            table_option_char_set(r##" CHARSET=utf8mb4"##),
            Ok(("", "utf8mb4".to_string()))
        );
        assert_eq!(
            table_option_char_set(r##" CHARSET utf8mb4"##),
            Ok(("", "utf8mb4".to_string()))
        );
    }

    #[test]
    fn test_table_option_collate() {
        assert_eq!(
            table_option_collate(r##" COLLATE=utf8mb4_bin"##),
            Ok(("", "utf8mb4_bin".to_string()))
        );
        assert_eq!(
            table_option_collate(r##" default COLLATE=utf8mb4_bin"##),
            Ok(("", "utf8mb4_bin".to_string()))
        );
    }

    #[test]
    fn test_table_option_comment() {
        assert_eq!(
            table_option_comment(r##" comment='配置表'"##),
            Ok(("", "配置表".to_string()))
        );
    }

    #[test]
    fn test_parse_table_option() {
        let expect = TableOption {
            engine: "InnoDB".to_string(),
            charset: "utf8mb4".to_string(),
            collate: "utf8mb4_bin".to_string(),
            comment: "配置表".to_string(),
        };
        assert_eq!(
            parse_table_option(
                r##" ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin COMMENT='配置表'"##
            ),
            Ok(("", expect))
        );
    }

    #[test]
    fn test_parse_create_sql() {
        let input = r###"CREATE TABLE `demo_table_user` (
  `id` bigint NOT NULL COMMENT '主键',
  `creator` bigint NOT NULL DEFAULT '0' COMMENT '创建人',
  `create_time` datetime NOT NULL DEFAULT CURRENT_TIMESTAMP COMMENT '创建时间',
  `updator` bigint NOT NULL DEFAULT '0' COMMENT '更新人',
  `update_time` datetime NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP COMMENT '更新时间',
  `version` int NOT NULL DEFAULT '1' COMMENT '乐观锁',
  `del_flag` tinyint NOT NULL DEFAULT '2' COMMENT '是否删除,1是,2否',
  PRIMARY KEY (`id`) USING BTREE,
  KEY `relate_idx` (`relate_id`) USING BTREE,
  KEY `tpl_id_idx` (`tpl_id`) USING BTREE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin COMMENT='模板分类关联表';"###;
        let res = parse_create_sql(input);
        assert_eq!(
            res.unwrap().1,
            TableSchema {
                table_name: "demo_table_user".to_string(),
                column_arr: vec![
                    OneColumn {
                        name: "id".to_string(),
                        typ: DataTypeEnum::Bigint,
                        comment: "主键".to_string(),
                    },
                    OneColumn {
                        name: "creator".to_string(),
                        typ: DataTypeEnum::Bigint,
                        comment: "创建人".to_string(),
                    },
                    OneColumn {
                        name: "create_time".to_string(),
                        typ: DataTypeEnum::DateTime(0,),
                        comment: "创建时间".to_string(),
                    },
                    OneColumn {
                        name: "updator".to_string(),
                        typ: DataTypeEnum::Bigint,
                        comment: "更新人".to_string(),
                    },
                    OneColumn {
                        name: "update_time".to_string(),
                        typ: DataTypeEnum::DateTime(0,),
                        comment: "更新时间".to_string(),
                    },
                    OneColumn {
                        name: "version".to_string(),
                        typ: DataTypeEnum::Int,
                        comment: "乐观锁".to_string(),
                    },
                    OneColumn {
                        name: "del_flag".to_string(),
                        typ: DataTypeEnum::TinyInt,
                        comment: "是否删除,1是,2否".to_string(),
                    },
                ],
                option: TableOption {
                    engine: "InnoDB".to_string(),
                    charset: "utf8mb4".to_string(),
                    collate: "utf8mb4_bin".to_string(),
                    comment: "模板分类关联表".to_string(),
                },
            }
        );
    }
}
