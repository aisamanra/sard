use lib_ruby_parser as p;
use std::collections::HashMap;

#[derive(Debug)]
pub struct Sig<'a> {
    pub params: HashMap<&'a str, Type<'a>>,
    pub returns: Option<Type<'a>>,
}

impl<'a> Sig<'a> {
    fn extract_params(&mut self, kwargs: &'a p::nodes::Kwargs) {
        for pair in kwargs.pairs.iter() {
            if let p::Node::Pair(p::nodes::Pair { key, value, .. }) = pair {
                let k = if let p::Node::Sym(sym) = key.as_ref() { sym } else { continue };
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
                },
                "returns" => {
                    if let Some(arg) = send.args.first() {
                        sig.returns = Some(Type::from_node(arg));
                    }
                },
                "void" =>
                    sig.returns = None,
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
