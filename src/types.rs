use lib_ruby_parser as p;

#[derive(Debug)]
pub struct Sig<'a> {
    pub params: Vec<Type<'a>>,
    pub returns: Option<Type<'a>>,
}

#[derive(Debug)]
pub struct Type<'a> {
    pub node: &'a p::Node,
}
