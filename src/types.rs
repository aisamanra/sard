use lib_ruby_parser as p;
use std::collections::HashMap;

/// A representation of a thing in Ruby which can have a name and
/// which we'll want to include in the `Sard` output.
#[derive(Debug)]
pub enum NamedItem<'a> {
    /// A Ruby class created with the `class` keyword
    Class(&'a p::nodes::Node, &'a p::nodes::Class),
    /// A Ruby module created with the `module` keyword
    Module(&'a p::nodes::Node, &'a p::nodes::Module),
    /// A plain method definition, optionally with associated `sig`
    Def(&'a p::nodes::Def, Option<Sig<'a>>),
    /// A static method definition, optionally with associated `sig`
    Defs(&'a p::nodes::Defs, Option<Sig<'a>>),
    /// A getter/setter method defined with `attr_`, optionally with
    /// associated `sig`
    Attr(
        AttrType,
        &'a p::nodes::Send,
        &'a p::nodes::Sym,
        Option<Sig<'a>>,
    ),
    /// A getter/setter method defined with Sorbet-compatible `prop`
    /// or `const`, optionally with associated `sig`
    Prop(PropType, &'a p::nodes::Send, &'a p::nodes::Sym, &'a p::Node),
    /// A constant assignment
    Casgn(&'a p::nodes::Casgn, Option<Type<'a>>),
}

impl<'a> NamedItem<'a> {
    pub fn to_string(&self) -> String {
        match self {
            NamedItem::Class(name, _) => format!("class {}", const_name(name)),
            NamedItem::Module(name, _) => format!("module {}", const_name(name)),
            NamedItem::Def(name, _) => format!("def {}", name.name),
            NamedItem::Defs(name, _) => format!("def self.{}", name.name),
            NamedItem::Attr(typ, _, name, _) => format!(
                "attr_{} {}",
                match typ {
                    AttrType::Reader => "reader",
                    AttrType::Writer => "writer",
                    AttrType::Accessor => "accessor",
                },
                name.name.bytes.to_string_lossy(),
            ),
            NamedItem::Prop(typ, _, name, _) => format!(
                "{} {}",
                match typ {
                    PropType::Prop => "prop",
                    PropType::Const => "const",
                },
                name.name.bytes.to_string_lossy(),
            ),
            NamedItem::Casgn(casgn, _) => format!("{}", casgn.name),
        }
    }
}

fn const_name<'a>(mut node: &'a p::Node) -> String {
    let mut parts: Vec<&'a String> = Vec::new();
    while let p::Node::Const(cnst) = node {
        parts.push(&cnst.name);
        if let Some(ref n) = cnst.scope {
            node = n;
        } else {
            break;
        }
    }
    parts.reverse();
    let mut name = String::new();
    for i in parts {
        name.push_str("::");
        name.push_str(i);
    }
    name
}

#[derive(Debug)]
pub enum AttrType {
    Reader,
    Writer,
    Accessor,
}

#[derive(Debug)]
pub enum PropType {
    Prop,
    Const,
}

#[derive(Debug)]
pub struct Sig<'a> {
    pub params: HashMap<&'a str, Type<'a>>,
    pub returns: Option<Type<'a>>,
}

impl<'a> Sig<'a> {
    fn extract_params(&mut self, kwargs: &'a p::nodes::Kwargs) {
        for pair in kwargs.pairs.iter() {
            if let p::Node::Pair(p::nodes::Pair { key, value, .. }) = pair {
                let k = if let p::Node::Sym(sym) = key.as_ref() {
                    sym
                } else {
                    continue;
                };
                let v = Type::from_node(value.as_ref());
                self.params.insert(k.name.bytes.as_str_lossy().unwrap(), v);
            }
        }
    }

    pub fn parse_sig(send: &'a p::Node) -> Option<Sig<'a>> {
        let mut node = send;
        let mut sig = Sig {
            params: HashMap::new(),
            returns: None,
        };
        while let p::Node::Send(send) = node {
            match send.method_name.as_ref() {
                "params" => {
                    if let Some(p::Node::Kwargs(kwargs)) = send.args.last() {
                        sig.extract_params(kwargs)
                    }
                }
                "returns" => {
                    if let Some(arg) = send.args.first() {
                        sig.returns = Some(Type::from_node(arg));
                    }
                }
                "void" => sig.returns = None,
                _ => (),
            }
            if let Some(n) = &send.recv {
                node = n.as_ref();
            } else {
                break;
            }
        }
        Some(sig)
    }
}

#[derive(Debug)]
pub struct Type<'a> {
    pub node: &'a p::Node,
}

impl<'a> Type<'a> {
    fn from_node(node: &'a p::Node) -> Type<'a> {
        Type { node }
    }
}
