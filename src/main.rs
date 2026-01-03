use serde::Deserialize;
use serde_json::json;
use grug_soa::LoadPrototype;

#[derive(Default, Clone, Deserialize)]
pub struct Foo {
    foo_field: String,
}

#[derive(Clone, Deserialize)]
pub struct Bar {
    bar_field: i32,
}

impl Default for Bar {
    fn default() -> Self {
        Bar { bar_field: 69 }
    }
}

#[derive(Clone, Deserialize, Default)]
pub struct Baz {
    a: String,
    inner: BazInner,
}

#[derive(Default, Clone, Deserialize)]
pub struct BazInner {
    b: String,
    c: f32,
}

#[derive(Default)]
pub struct MySoA {
    foo: Vec<Foo>,
    bar: Vec<Bar>,
    baz: Vec<Baz>,
    num: Vec<i32>,
    name: Vec<String>,
    soa_ignored_field: String,
}

grug_soa::impl_load_prototype!(MySoA { foo: Foo, bar: Bar, baz: Baz, num: i32, name: String });

// you can have fields you choose not to register also (if other fields belonged in it)
// an alternative implementation could be like define_soa { foo bar baz etc }
// but i think its more magic

fn main() {
    // Load time
    let mut prototype_soa = MySoA::default();

    prototype_soa.load_prototype(json!({
        "foo": {
            "foo_field": "foofield value"
        },
        "num": 1337,
        "name": "grugname1"
    }));

    prototype_soa.load_prototype(json!({
        "foo": {
            "foo_field": "asdf"
        },
        "bar": {
            "bar_field": 420
        },
        "baz": {
            "a": "bazfield value",
            "inner": {
                "b": "bazinnerfield value",
                "c": 420.69
            }
        },
        "num": 696969,
        "name": "grugname2"
    }));

    assert_eq!(prototype_soa.foo[0].foo_field, "foofield value");
    assert_eq!(prototype_soa.bar[0].bar_field, 69);
    assert_eq!(prototype_soa.baz[0].a, "");
    assert_eq!(prototype_soa.baz[0].inner.b, "");
    assert!((prototype_soa.baz[0].inner.c - 0.0).abs() < 1e-6);
    assert_eq!(prototype_soa.num[0], 1337);
    assert_eq!(prototype_soa.name[0], "grugname1");
    assert_eq!(prototype_soa.foo[1].foo_field, "asdf");
    assert_eq!(prototype_soa.bar[1].bar_field, 420);
    assert_eq!(prototype_soa.baz[1].a, "bazfield value");
    assert_eq!(prototype_soa.baz[1].inner.b, "bazinnerfield value");
    assert!((prototype_soa.baz[1].inner.c - 420.69).abs() < 1e-6);
    assert_eq!(prototype_soa.num[1], 696969);
    assert_eq!(prototype_soa.name[1], "grugname2");

    // runtime - no deserialization happening
    let mut runtime_soa = MySoA::default();
    // spawn some entities
    runtime_soa.spawn_entity(&prototype_soa, 1);
    runtime_soa.spawn_entity(&prototype_soa, 1);
    runtime_soa.spawn_entity(&prototype_soa, 0);

    for i in 0..runtime_soa.foo.len() {
        println!("entity: {}", i);
        println!("foo: {}", runtime_soa.foo[i].foo_field);
        println!("bar: {}", runtime_soa.bar[i].bar_field);
        println!("baz: {}", runtime_soa.baz[i].a);
        println!("baz inner: {}", runtime_soa.baz[i].inner.b);
        println!("baz inner: {}", runtime_soa.baz[i].inner.c);
        println!("num: {}", runtime_soa.num[i]);
        println!("name: {}", runtime_soa.name[i]);
    }
}