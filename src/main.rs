use lib_ruby_parser as p;

mod definitions;
mod types;

const SAMPLE: &'static [u8] = include_bytes!("../sample/sample.rb");

fn main() {
    let parser = p::Parser::new(SAMPLE, std::default::Default::default());
    let p::ParserResult { ast, comments, .. } = parser.do_parse();
    let ast = ast.unwrap();

    let defn = definitions::Definitions::new(&ast);
    for f in defn {
        println!("Defn: {:?}", f.to_string());
    }

    for f in comments {
        println!(
            "C: {:?}: {:?}",
            f,
            std::str::from_utf8(&SAMPLE[f.location.begin..f.location.end])
        );
    }
}
