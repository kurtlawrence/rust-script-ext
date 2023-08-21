use proc_macro::{Delimiter, Group, Ident, Literal, Punct, Spacing, Span, TokenStream, TokenTree};
use std::str::FromStr;

#[proc_macro]
pub fn cargs(stream: TokenStream) -> TokenStream {
    if stream.is_empty() {
        TokenStream::from_str("{ let x: [String; 0] = []; x }").expect("valid Rust")
    } else {
        let mut buf = Vec::new();

        let mut stream = stream.into_iter();
        let stream = stream.by_ref();

        while let Some(arg) = take_arg(stream) {
            buf.extend(arg);
            buf.push(Punct::new(',', proc_macro::Spacing::Alone).into());
        }

        TokenTree::from(Group::new(
            proc_macro::Delimiter::Bracket,
            TokenStream::from_iter(buf),
        ))
        .into()
    }
}

fn take_arg(stream: &mut dyn Iterator<Item = TokenTree>) -> Option<Vec<TokenTree>> {
    let f = stream.next()?;

    match f {
        // if encased in braces, the arg becomes { .. }.to_string()
        TokenTree::Group(g) if g.delimiter() == Delimiter::Brace => {
            let x = maybe_wrap_in_quotes(suffix_to_string(vec![g.into()]));
            expect_comma(stream.next());
            vec![x].into()
        }
        // encountered comma with no preceding arg
        TokenTree::Punct(p) if p.as_char() == ',' => {
            panic!("expected an argument, but found a comma")
        }
        TokenTree::Literal(l) => {
            let x = maybe_wrap_in_quotes(suffix_to_string(vec![l.into()]));
            expect_comma(stream.next());
            vec![x].into()
        }
        x => {
            // comma gets consumed with take_while
            let s = std::iter::once(x).chain(
                stream.take_while(|t| !matches!(t, TokenTree::Punct(p) if p.as_char() == ',')),
            );
            let mut s = TokenStream::from_iter(s).to_string();
            s.retain(|c| c != ' ');
            let s = TokenTree::Literal(Literal::string(&s));
            suffix_to_string(vec![s]).into()
        }
    }
}

fn suffix_to_string(mut ts: Vec<TokenTree>) -> Vec<TokenTree> {
    ts.extend(TokenStream::from_str(".to_string()").expect("valid Rust"));
    ts
}

fn expect_comma(tt: Option<TokenTree>) {
    // consume a comma
    if let Some(n) = tt {
        if !matches!(
                    n,
                    TokenTree::Punct(p) if p.as_char() == ',')
        {
            panic!("expecting a comma delimiter");
        }
    }
}

fn maybe_wrap_in_quotes(expr: Vec<TokenTree>) -> TokenTree {
    let mut buf = TokenStream::from_str("let x = ").expect("valid Rust");
    buf.extend(expr);
    buf.extend([semi_colon()]);

    buf.extend(TokenStream::from_str("if x.contains(' ')").expect("valid Rust"));
    buf.extend([
        TokenTree::Group(Group::new(
            Delimiter::Brace,
            TokenStream::from_str(r#""\"".to_string() + &x + "\"""#).expect("valid Rust"),
        )),
        ident("else"),
        TokenTree::Group(Group::new(
            Delimiter::Brace,
            TokenStream::from_iter([ident("x")]),
        )),
    ]);

    TokenTree::Group(Group::new(Delimiter::Brace, buf))
}

#[proc_macro]
pub fn cmd(stream: TokenStream) -> TokenStream {
    let mut stream = stream.into_iter();
    let stream = stream.by_ref();

    let program = stream.take_while(|t| !matches!(t, TokenTree::Punct(p) if p.as_char() == ':'));
    let program = TokenStream::from_iter(program).to_string().replace(' ', "");
    let program = TokenTree::Literal(Literal::string(&program));

    let args = cargs(TokenStream::from_iter(stream));

    let mut stream =
        TokenStream::from_str("let mut x = ::std::process::Command::new").expect("valid Rust");

    stream.extend([
        TokenTree::Group(Group::new(Delimiter::Parenthesis, program.into())),
        semi_colon(),
    ]);
    stream.extend(TokenStream::from_str("x.args"));
    stream.extend([
        TokenTree::Group(Group::new(Delimiter::Parenthesis, args)),
        semi_colon(),
        ident("x"),
    ]);

    TokenTree::Group(Group::new(Delimiter::Brace, stream)).into()
}

fn semi_colon() -> TokenTree {
    TokenTree::Punct(Punct::new(';', Spacing::Alone))
}

fn ident(s: &str) -> TokenTree {
    TokenTree::Ident(Ident::new(s, Span::call_site()))
}
