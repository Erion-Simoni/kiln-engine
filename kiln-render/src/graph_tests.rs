use super::*;

fn tex(name: &str, transient: bool) -> ResourceDesc {
    ResourceDesc {
        name: name.into(),
        kind: ResourceKind::Texture {
            width: 64,
            height: 64,
            format: TextureFormatKey::Rgba8Unorm,
        },
        transient,
    }
}

#[test]
fn topo_orders_producer_before_consumer() {
    let mut g = RenderGraph::new();
    let a = g.add_resource(tex("a", true));
    let b = g.add_resource(tex("b", true));
    g.add_pass(PassDecl {
        name: "write_a".into(),
        resources: vec![ResourceRef {
            id: a,
            usage: ResourceUsage::Write,
        }],
    });
    g.add_pass(PassDecl {
        name: "read_a_write_b".into(),
        resources: vec![
            ResourceRef {
                id: a,
                usage: ResourceUsage::Read,
            },
            ResourceRef {
                id: b,
                usage: ResourceUsage::Write,
            },
        ],
    });
    let c = g.compile().unwrap();
    assert_eq!(c.ordered_pass_names(), vec!["write_a", "read_a_write_b"]);
}

#[test]
fn non_overlapping_transients_alias() {
    let mut g = RenderGraph::new();
    let a = g.add_resource(tex("a", true));
    let b = g.add_resource(tex("b", true));
    g.add_pass(PassDecl {
        name: "p0".into(),
        resources: vec![ResourceRef {
            id: a,
            usage: ResourceUsage::Write,
        }],
    });
    g.add_pass(PassDecl {
        name: "p1".into(),
        resources: vec![ResourceRef {
            id: b,
            usage: ResourceUsage::Write,
        }],
    });
    let c = g.compile().unwrap();
    assert!(c.aliases(a, b), "A and B should share a physical slot");
    assert_eq!(c.slot_count, 1);
}

#[test]
fn overlapping_transients_do_not_alias() {
    let mut g = RenderGraph::new();
    let a = g.add_resource(tex("a", true));
    let b = g.add_resource(tex("b", true));
    g.add_pass(PassDecl {
        name: "both".into(),
        resources: vec![
            ResourceRef {
                id: a,
                usage: ResourceUsage::Write,
            },
            ResourceRef {
                id: b,
                usage: ResourceUsage::Write,
            },
        ],
    });
    let c = g.compile().unwrap();
    assert!(!c.aliases(a, b));
    assert_eq!(c.slot_count, 2);
}

#[test]
fn external_not_aliased_with_transient() {
    let mut g = RenderGraph::new();
    let surf = g.add_resource(ResourceDesc {
        name: "surface".into(),
        kind: ResourceKind::Texture {
            width: 64,
            height: 64,
            format: TextureFormatKey::Surface,
        },
        transient: false,
    });
    let tmp = g.add_resource(tex("tmp", true));
    g.add_pass(PassDecl {
        name: "p0".into(),
        resources: vec![ResourceRef {
            id: tmp,
            usage: ResourceUsage::Write,
        }],
    });
    g.add_pass(PassDecl {
        name: "p1".into(),
        resources: vec![
            ResourceRef {
                id: tmp,
                usage: ResourceUsage::Read,
            },
            ResourceRef {
                id: surf,
                usage: ResourceUsage::Write,
            },
        ],
    });
    let c = g.compile().unwrap();
    assert!(!c.aliases(surf, tmp));
}
