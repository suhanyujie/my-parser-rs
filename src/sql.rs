/*
>* 文章名称：从零编写一个解析器（2）—— 解析 MySQL 建表语句
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

## 参考
* https://dev.mysql.com/doc/refman/8.0/en/create-table.html

*/

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum SqlType {
    Bool,
    Char(u16),
    Varchar(u16),
    Int(u16),
    UnsignedInt(u16),
    Bigint(u16),
    UnsignedBigint(u16),
    Tinyint(u16),
    UnsignedTinyint(u16),
    Blob,
    Longblob,
    Mediumblob,
    Tinyblob,
    Double,
    Float,
    Real,
    Tinytext,
    Mediumtext,
    Longtext,
    Text,
    Date,
    DateTime(u16),
    Timestamp,
    Binary(u16),
    Varbinary(u16),
    Enum(Vec<Literal>),
    Decimal(u8, u8),
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum Literal {
    Null,
    Integer(i64),
    UnsignedInteger(u64),
    FixedPoint(Real),
    String(String),
    Blob(Vec<u8>),
    CurrentTime,
    CurrentDate,
    CurrentTimestamp,
    Placeholder(ItemPlaceholder),
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum ItemPlaceholder {
    QuestionMark,
    DollarNumber(i32),
    ColonNumber(i32),
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Real {
    pub integral: i32,
    pub fractional: i32,
}

fn parse_sql1() {}

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
}
