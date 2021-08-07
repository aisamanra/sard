use crate::types::{AttrType, NamedItem, PropType, Sig};
use lib_ruby_parser as p;

/// An iterator which can walk over a parsed Ruby AST and
/// incrementally yield `NamedItem` values. This keeps an internal
/// stack of what to look at next, which it lazily initializes and
/// pops as it walks over the file.
pub struct Definitions<'a> {
    stashed_sig: Option<Sig<'a>>,
    context: Vec<NamedItem<'a>>,
}

impl<'a> Definitions<'a> {
    /// Create a new `Definitions` iterator with the provided AST root
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
    /// Push the provided node (or subparts of the node) onto the
    /// stack to provide later. This is where nodes are 'decoded' into
    /// our internal representation, but also where recursion over
    /// nodes happens.
    fn push_next(&mut self, node: &'a p::Node) {
        match node {
            // class-like nodes:
            p::Node::Module(module) => {
                self.stashed_sig = None;
                self.context.push(NamedItem::Module(&module.name, module));
            }
            p::Node::Class(class) => {
                self.stashed_sig = None;
                self.context.push(NamedItem::Class(&class.name, class));
            }
            // def-like nodes:
            p::Node::Def(def) => {
                // if we've already seen a sig, then this will remove
                // it; otherwise this will leave the `None` intact
                let sig = std::mem::replace(&mut self.stashed_sig, None);
                self.context.push(NamedItem::Def(def, sig));
            }
            p::Node::Defs(defs) => {
                let sig = std::mem::replace(&mut self.stashed_sig, None);
                self.context.push(NamedItem::Defs(defs, sig));
            }
            // constant assignment
            p::Node::Casgn(casgn) => {
                self.context.push(NamedItem::Casgn(casgn, None));
            }
            // not all sends that we process will define methods, but
            // some will
            p::Node::Send(send) => {
                if let Some(node) = self.known_defining_method(&send) {
                    self.context.push(node);
                }
            }
            // groups of statements: we recurse over these eagerly
            p::Node::Begin(p::nodes::Begin {
                statements: stmts, ..
            }) => {
                for s in stmts {
                    self.push_next(s);
                }
            }
            // we mostly only care about well-formed `sig` blocks here
            // TODO: handle enum blocks
            p::Node::Block(p::nodes::Block {
                call,
                body: Some(body),
                ..
            }) => {
                match call.as_ref() {
                    p::Node::Send(p::nodes::Send { method_name, .. }) if method_name == "sig" => (),
                    _ => return,
                }
                self.stashed_sig = Sig::parse_sig(body);
            }
            _ => (),
        }
    }

    /// this will be called once we've removed an item from the stack:
    /// if that item itself contains other things we care about, we
    /// should handle recursing into them here.
    fn push_children(&mut self, item: &NamedItem<'a>) {
        match item {
            NamedItem::Class(_, p::nodes::Class { body: Some(n), .. }) => self.push_next(n),
            NamedItem::Module(_, p::nodes::Module { body: Some(n), .. }) => self.push_next(n),
            _ => (),
        }
    }

    /// This tries to produce a `NamedItem` from a send if it's a send
    /// that defines a method in a way that we know about.
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
            "attr_reader" => Some(NamedItem::Attr(
                AttrType::Reader,
                send,
                name,
                std::mem::replace(&mut self.stashed_sig, None),
            )),
            "attr_writer" => Some(NamedItem::Attr(
                AttrType::Writer,
                send,
                name,
                std::mem::replace(&mut self.stashed_sig, None),
            )),
            "attr_accessor" => Some(NamedItem::Attr(
                AttrType::Accessor,
                send,
                name,
                std::mem::replace(&mut self.stashed_sig, None),
            )),
            "prop" => Some(NamedItem::Prop(PropType::Prop, send, name, &send.args[1])),
            "const" => Some(NamedItem::Prop(PropType::Const, send, name, &send.args[1])),
            _ => None,
        }
    }
}

impl<'a> Iterator for Definitions<'a> {
    type Item = NamedItem<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        // the implementation here is pretty straightforward, and
        // mostly handled in `push_next` above. We've got a queue: pop
        // the next thing, unless it's empty, in which case return
        // `None`.
        let item = match self.context.pop() {
            None => return None,
            Some(x) => x,
        };
        // that thing might itself contain other stuff: push anything
        // it has
        self.push_children(&item);
        // and return the thing
        Some(item)
    }
}
