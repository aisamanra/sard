use lib_ruby_parser as p;

mod types;

const SAMPLE: &'static [u8] = include_bytes!("../sample/sample.rb");

fn main() {
    let parser = p::Parser::new(SAMPLE, std::default::Default::default());
    let p::ParserResult { ast, comments, .. } = parser.do_parse();
    let ast = ast.unwrap();

    let mut defn = DefnIter {
        stashed_sig: None,
        context: vec![],
    };
    defn.push_next(&ast);
    for f in defn {
        println!("Defn: {:?}", f);
    }

    for f in comments {
        println!("C: {:?}: {:?}", f, std::str::from_utf8(&SAMPLE[f.location.begin..f.location.end]));
    }
}

struct DefnIter<'a> {
    stashed_sig: Option<types::Sig<'a>>,
    context: Vec<NamedItem<'a>>,
}

#[derive(Debug)]
enum NamedItem<'a> {
    Class(&'a p::nodes::Node, &'a p::nodes::Class),
    Module(&'a p::nodes::Node, &'a p::nodes::Module),
    Def(&'a p::nodes::Def, Option<types::Sig<'a>>),
    Defs(&'a p::nodes::Defs, Option<types::Sig<'a>>),
    Attr(AttrType, &'a p::nodes::Send, &'a p::nodes::Sym, Option<types::Sig<'a>>),
    Prop(PropType, &'a p::nodes::Send, &'a p::nodes::Sym, &'a p::Node),
    Casgn(&'a p::nodes::Casgn),
}

#[derive(Debug)]
enum AttrType {
    Reader,
    Writer,
    Accessor,
}

#[derive(Debug)]
enum PropType {
    Prop,
    Const,
}

impl<'a> NamedItem<'a> {
    fn to_string(&self) -> String {
        match self {
            NamedItem::Class(name, _) =>
                format!("class {}", const_name(name)),
            NamedItem::Module(name, _) =>
                format!("module {}", const_name(name)),
            NamedItem::Def(name, _) =>
                format!("def {}", name.name),
            NamedItem::Defs(name, _) =>
                format!("def self.{}", name.name),
            NamedItem::Attr(typ, _, name, _) =>
                format!(
                    "attr_{} {}",
                    match typ {
                        AttrType::Reader => "reader",
                        AttrType::Writer => "writer",
                        AttrType::Accessor => "accessor",
                    },
                    name.name.bytes.to_string_lossy(),
                ),
            NamedItem::Prop(typ, _, name, _) =>
                format!(
                    "{} {}",
                    match typ {
                        PropType::Prop => "prop",
                        PropType::Const => "const",
                    },
                    name.name.bytes.to_string_lossy(),
                ),
            NamedItem::Casgn(casgn) =>
                format!("{}", casgn.name),
        }
    }
}

impl<'a> DefnIter<'a> {
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
                self.stashed_sig = types::Sig::parse_sig(body);
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

impl<'a> Iterator for DefnIter<'a> {
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
