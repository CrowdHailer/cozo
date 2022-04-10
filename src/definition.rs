use pest::iterators::{Pair, Pairs};
use crate::ast::parse_string;
use crate::env::Env;
use crate::error::Result;
use crate::error::CozoError::*;
use crate::parser::{Rule};
use crate::typing::{Col, Edge, Node, Structured, StructuredEnv, StructuredEnvItem, TableId, Typing};
use crate::typing::Persistence::{Global, Local};
use crate::typing::StorageStatus::Planned;
use crate::value::Value;

fn parse_ident(pair: Pair<Rule>) -> String {
    pair.as_str().to_string()
}

fn build_name_in_def(pair: Pair<Rule>, forbid_underscore: bool) -> Result<String> {
    let inner = pair.into_inner().next().unwrap();
    let name = match inner.as_rule() {
        Rule::ident => parse_ident(inner),
        Rule::raw_string | Rule::s_quoted_string | Rule::quoted_string => parse_string(inner)?,
        _ => unreachable!()
    };
    if forbid_underscore && name.starts_with('_') {
        Err(ReservedIdent)
    } else {
        Ok(name)
    }
}

fn parse_col_name(pair: Pair<Rule>) -> Result<(String, bool)> {
    let mut pairs = pair.into_inner();
    let mut is_key = false;
    let mut nxt_pair = pairs.next().unwrap();
    if nxt_pair.as_rule() == Rule::key_marker {
        is_key = true;
        nxt_pair = pairs.next().unwrap();
    }

    Ok((build_name_in_def(nxt_pair, true)?, is_key))
}


impl StructuredEnvItem {
    pub fn build_edge_def(&mut self, pair: Pair<Rule>, table_id: TableId) -> Result<()> {
        let mut inner = pair.into_inner();
        let src_name = build_name_in_def(inner.next().unwrap(), true)?;
        let src = self.resolve(&src_name).ok_or(UndefinedType)?;
        let src_id = if let Structured::Node(n, _) = src {
            n.id
        } else {
            return Err(WrongType);
        };
        let name = build_name_in_def(inner.next().unwrap(), true)?;
        let dst_name = build_name_in_def(inner.next().unwrap(), true)?;
        let dst = self.resolve(&dst_name).ok_or(UndefinedType)?;
        let dst_id = if let Structured::Node(n, _) = dst {
            n.id
        } else {
            return Err(WrongType);
        };
        if table_id.0 == Global && (src_id.0 == Local || dst_id.0 == Local) {
            return Err(IncompatibleEdge);
        }
        let (keys, cols) = if let Some(p) = inner.next() {
            self.build_col_defs(p)?
        } else {
            (vec![], vec![])
        };
        let edge = Edge {
            src: src_id,
            dst: dst_id,
            id: table_id,
            keys,
            cols,
        };
        if self.define_new(name.to_string(), Structured::Edge(edge, Planned)) {
            if let Some(Structured::Node(src, _)) = self.resolve_mut(&src_name) {
                src.out_e.push(table_id);
            } else {
                unreachable!()
            }

            if let Some(Structured::Node(dst, _)) = self.resolve_mut(&dst_name) {
                dst.in_e.push(table_id);
            } else {
                unreachable!()
            }
            Ok(())
        } else {
            Err(NameConflict)
        }
    }
    pub fn build_node_def(&mut self, pair: Pair<Rule>, table_id: TableId) -> Result<()> {
        let mut inner = pair.into_inner();
        let name = build_name_in_def(inner.next().unwrap(), true)?;
        let (keys, cols) = self.build_col_defs(inner.next().unwrap())?;
        let node = Node {
            id: table_id,
            keys,
            cols,
            out_e: vec![],
            in_e: vec![],
            attached: vec![],
        };
        if self.define_new(name.to_string(), Structured::Node(node, Planned)) {
            Ok(())
        } else {
            Err(NameConflict)
        }
    }

    fn build_type(&self, pair: Pair<Rule>) -> Result<Typing> {
        let mut pairs = pair.into_inner();
        let mut inner = pairs.next().unwrap();
        let nullable = if Rule::nullable_marker == inner.as_rule() {
            inner = pairs.next().unwrap();
            true
        } else {
            false
        };
        let t = match inner.as_rule() {
            Rule::simple_type => {
                let name = parse_ident(inner.into_inner().next().unwrap());
                if let Some(Structured::Typing(t)) = self.resolve(&name) {
                    t.clone()
                } else {
                    return Err(UndefinedType);
                }
            }
            Rule::list_type => {
                let inner_t = self.build_type(inner.into_inner().next().unwrap())?;
                Typing::HList(Box::new(inner_t))
            }
            // Rule::tuple_type => {},
            _ => unreachable!()
        };
        Ok(if nullable {
            Typing::Nullable(Box::new(t))
        } else {
            t
        })
    }

    fn build_default_value(&self, _pair: Pair<Rule>) -> Result<Value<'static>> {
        // TODO: _pair is an expression, parse it and evaluate it to a constant value
        Ok(Value::Null)
    }

    fn build_col_entry(&self, pair: Pair<Rule>) -> Result<(Col, bool)> {
        let mut pairs = pair.into_inner();
        let (name, is_key) = parse_col_name(pairs.next().unwrap())?;
        let typ = self.build_type(pairs.next().unwrap())?;
        let default = if let Some(p) = pairs.next() {
            // TODO: check value is suitable for the type
            Some(self.build_default_value(p)?)
        } else {
            None
        };
        Ok((Col {
            name,
            typ,
            default,
        }, is_key))
    }

    fn build_col_defs(&self, pair: Pair<Rule>) -> Result<(Vec<Col>, Vec<Col>)> {
        let mut keys = vec![];
        let mut cols = vec![];
        for pair in pair.into_inner() {
            let (col, is_key) = self.build_col_entry(pair)?;
            if is_key {
                keys.push(col)
            } else {
                cols.push(col)
            }
        }

        Ok((keys, cols))
    }
}

impl StructuredEnv {
    pub fn build_table(&mut self, pairs: Pairs<Rule>) -> Result<()> {
        for pair in pairs {
            match pair.as_rule() {
                r @ (Rule::global_def | Rule::local_def) => {
                    let inner = pair.into_inner().next().unwrap();
                    let is_local = r == Rule::local_def;
                    let next_id = self.get_next_table_id(is_local);
                    let env_to_build = if is_local {
                        self.root_mut()
                    } else {
                        self.cur_mut()
                    };

                    // println!("{:?} {:?}", r, inner.as_rule());
                    match inner.as_rule() {
                        Rule::node_def => {
                            env_to_build.build_node_def(inner, next_id)?;
                        }
                        Rule::edge_def => {
                            env_to_build.build_edge_def(inner, next_id)?;
                        }
                        _ => todo!()
                    }
                }
                Rule::EOI => {}
                _ => unreachable!()
            }
        }
        Ok(())
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use pest::Parser as PestParser;
    use crate::parser::Parser;

    #[test]
    fn definitions() {
        let s = r#"
            local node "Person" {
                *id: Int,
                name: String,
                email: ?String,
                habits: ?[?String]
            }

            local edge (Person)-[Friend]->(Person) {
                relation: ?String
            }
        "#;
        let parsed = Parser::parse(Rule::file, s).unwrap();
        let mut env = StructuredEnv::new();
        env.build_table(parsed).unwrap();
        println!("{:#?}", env.resolve("Person"));
        println!("{:#?}", env.resolve("Friend"));
    }
}