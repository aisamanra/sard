use lib_ruby_parser as p;
use crate::types::{NamedItem, AttrType, PropType, Sig};

pub struct Definitions<'a> {
    stashed_sig: Option<Sig<'a>>,
    context: Vec<NamedItem<'a>>,
}

impl<'a> Definitions<'a> {
    pub fn new(root: &'a p::Node) -> Definitions<'a> {
        let mut iter = Definitions {
            stashed_sig: None,
            context: vec![],
        };
        iter.push_next(root);
        iter
    }
}

impl<'a> Definitions<'a> {
    fn push_next(&mut self, node: &'a p::Node) {
        match node {
            p::Node::Module(module) => {
                self.stashed_sig = None;
                self.context.push(NamedItem::Module(&module.name, module));
            },
            p::Node::Class(class) => {
                self.stashed_sig = None;
                self.context.push(NamedItem::Class(&class.name, class));
            },
            p::Node::Def(def) => {
                let sig = std::mem::replace(&mut self.stashed_sig, None);
                self.context.push(NamedItem::Def(def, sig));
            },
            p::Node::Defs(defs) => {
                let sig = std::mem::replace(&mut self.stashed_sig, None);
                self.context.push(NamedItem::Defs(defs, sig));
            },
            p::Node::Casgn(casgn) => {
                self.context.push(NamedItem::Casgn(casgn));
            },
            p::Node::Send(send) => {
                if let Some(node) = self.known_defining_method(&send) {
                    self.context.push(node);
                }
            },
            p::Node::Begin(p::nodes::Begin {statements: stmts, ..}) => {
                for s in stmts {
                    self.push_next(s);
                }
            },
            p::Node::Block(p::nodes::Block {call, body: Some(body), ..}) => {
                match call.as_ref() {
                    p::Node::Send( p::nodes::Send { method_name, .. }) if method_name == "sig" => (),
                    _ => return,
                }
                self.stashed_sig = Sig::parse_sig(body);
            },
            _ => (),
        }
    }

    fn push_children(&mut self, item: &NamedItem<'a>) {
        match item {
            NamedItem::Class(_, p::nodes::Class { body: Some(n), .. }) =>
                self.push_next(n),
            NamedItem::Module(_, p::nodes::Module { body: Some(n), .. }) =>
                self.push_next(n),
            _ => (),
        }
    }

    fn known_defining_method(&mut self, send: &'a p::nodes::Send) -> Option<NamedItem<'a>> {
        if send.args.len() < 1 {
            return None;
        }
        let name = if let p::Node::Sym(ref name) = send.args[0] {
            name
        } else {
            return None;
        };
        match send.method_name.as_ref() {
            "attr_reader" =>
                Some(NamedItem::Attr(AttrType::Reader, send, name, std::mem::replace(&mut self.stashed_sig, None))),
            "attr_writer" =>
                Some(NamedItem::Attr(AttrType::Writer, send, name, std::mem::replace(&mut self.stashed_sig, None))),
            "attr_accessor" =>
                Some(NamedItem::Attr(AttrType::Accessor, send, name, std::mem::replace(&mut self.stashed_sig, None))),
            "prop" =>
                Some(NamedItem::Prop(PropType::Prop, send, name, &send.args[1])),
            "const" =>
                Some(NamedItem::Prop(PropType::Const, send, name, &send.args[1])),
            _ => None,
        }
    }

}

impl<'a> Iterator for Definitions<'a> {
    type Item = NamedItem<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let item = match self.context.pop() {
            None => return None,
            Some(x) => x,
        };
        self.push_children(&item);
        Some(item)
    }
}
