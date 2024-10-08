use macros::*;

#[test]
fn cargs_expanding() {
    let a = cargs!();
    let e: [String; 0] = [];
    assert_eq!(a, e);

    let a = cargs!(hello, world);
    assert_eq!(a, ["hello".to_string(), "world".to_string()]);

    let w = "world";
    let a = cargs!(hello, { w });
    assert_eq!(a, ["hello".to_string(), "world".to_string()]);

    let a = cargs!("hello/path", { w });
    assert_eq!(a, ["hello/path".to_string(), "world".to_string()]);

    let a = cargs!(hello / path, { w });
    assert_eq!(a, ["hello/path".to_string(), "world".to_string()]);

    let a = cargs!("hello/path", { format!("W{w}") }, --flag);
    assert_eq!(
        a,
        [
            "hello/path".to_string(),
            "Wworld".to_string(),
            "--flag".to_string()
        ]
    );

    let a = cargs!(hello / path, "a literal",);
    assert_eq!(a, ["hello/path".to_string(), "\"a literal\"".to_string()]);
}

#[test]
fn cmd_smoketest() {
    let a = format!("{:?}", cmd!(ls));
    assert_eq!(&a, r#""ls""#);

    let a = format!("{:?}", cmd!(ls: foo, bar));
    assert_eq!(&a, r#""ls" "foo" "bar""#);

    let a = format!("{:?}", cmd!(ls: foo, bar/zog));
    assert_eq!(&a, r#""ls" "foo" "bar/zog""#);
}
