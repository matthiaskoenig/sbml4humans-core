//! Archive references are scoped by entry even when model and element IDs collide.
use sbml4humans_report::{SourceDocument, build_many};
fn document(body: &str, external: &str) -> sbml4humans_model::Document {
    let xml = format!(
        r#"<sbml xmlns="http://www.sbml.org/sbml/level3/version2/core" xmlns:comp="http://www.sbml.org/sbml/level3/version1/comp/version1" level="3" version="2" comp:required="true"><model id="shared">{body}</model><comp:listOfExternalModelDefinitions>{external}</comp:listOfExternalModelDefinitions></sbml>"#
    );
    sbml4humans_reader::read(xml.as_bytes()).unwrap()
}
#[test]
fn colliding_ids_keep_distinct_external_targets_without_importing_their_nodes() {
    let main = document(
        r#"<listOfParameters><parameter id="p" value="1" constant="true"><comp:listOfReplacedElements><comp:replacedElement comp:submodelRef="b" comp:idRef="p"/><comp:replacedElement comp:submodelRef="c" comp:idRef="p"/></comp:listOfReplacedElements></parameter></listOfParameters><comp:listOfSubmodels><comp:submodel comp:id="b" comp:modelRef="eb"/><comp:submodel comp:id="c" comp:modelRef="ec"/></comp:listOfSubmodels>"#,
        r#"<comp:externalModelDefinition comp:id="eb" comp:source="b.xml"/><comp:externalModelDefinition comp:id="ec" comp:source="c.xml"/>"#,
    );
    let child = document(
        r#"<listOfParameters><parameter id="p" value="2" constant="true"/></listOfParameters>"#,
        "",
    );
    let sources = [
        SourceDocument {
            location: "./a.xml",
            document: &main,
            checksum: None,
        },
        SourceDocument {
            location: "./b.xml",
            document: &child,
            checksum: None,
        },
        SourceDocument {
            location: "./c.xml",
            document: &child,
            checksum: None,
        },
    ];
    let reports = build_many(&sources, &[], 10000).unwrap();
    let targets: Vec<_> = reports[0]
        .link_graph
        .edges
        .iter()
        .filter_map(|e| {
            e.target_entry
                .as_ref()
                .filter(|_| e.target == "shared/Parameter:p")
        })
        .collect();
    assert_eq!(targets, ["./b.xml", "./c.xml"]);
    assert_eq!(reports[1].link_graph.nodes.len(), 3);
    assert!(reports[0].ode_error.is_none(), "{:?}", reports[0].ode_error);
}
#[test]
fn chains_cycles_checksums_and_non_sbml_locations_preserve_resolution_statuses() {
    let a = document(
        "",
        r#"<comp:externalModelDefinition comp:id="hop" comp:source="b.xml" comp:modelRef="last" comp:md5="ABCDEFABCDEFABCDEFABCDEFABCDEFABCD"/><comp:externalModelDefinition comp:id="cycle" comp:source="b.xml" comp:modelRef="cycle"/><comp:externalModelDefinition comp:id="readme" comp:source="README.md"/><comp:externalModelDefinition comp:id="outside" comp:source="../../escape.xml"/><comp:externalModelDefinition comp:id="remote" comp:source="file:///etc/passwd"/>"#,
    );
    let b = document(
        "",
        r#"<comp:externalModelDefinition comp:id="last" comp:source="child%20model.xml"/><comp:externalModelDefinition comp:id="cycle" comp:source="a.xml" comp:modelRef="cycle"/>"#,
    );
    let c = document("", "");
    let sources = [
        SourceDocument {
            location: "./models/a.xml",
            document: &a,
            checksum: None,
        },
        SourceDocument {
            location: "./models/b.xml",
            document: &b,
            checksum: Some("abcdefabcdefabcdefabcdefabcdefabcd"),
        },
        SourceDocument {
            location: "./models/child model.xml",
            document: &c,
            checksum: None,
        },
    ];
    let reports = build_many(&sources, &["./models/README.md"], 10000).unwrap();
    let definitions = &reports[0].external_model_definitions;
    assert_eq!(
        definitions[0]["resolution"],
        serde_json::json!({"status":"resolved","entry":"./models/child model.xml","model":"shared/Model:shared","md5Matches":true})
    );
    assert_eq!(definitions[1]["resolution"]["status"], "circular");
    assert_eq!(definitions[2]["resolution"]["status"], "notSbml");
    assert_eq!(definitions[3]["resolution"]["status"], "notFound");
    assert_eq!(definitions[4]["resolution"]["status"], "remoteSource");
}

#[test]
fn long_external_definition_chains_use_scoped_identifier_indices() {
    let mut definitions = String::new();
    for i in 0..600 {
        let target = if i == 599 {
            "shared".to_owned()
        } else {
            format!("e{}", i + 1)
        };
        definitions.push_str(&format!(r#"<comp:externalModelDefinition comp:id="e{i}" comp:source="a.xml" comp:modelRef="{target}"/>"#));
    }
    let model = document("", &definitions);
    let sources = [SourceDocument {
        location: "./a.xml",
        document: &model,
        checksum: None,
    }];
    let reports = build_many(&sources, &[], 10000).unwrap();
    assert_eq!(reports[0].external_model_definitions.len(), 600);
    assert!(
        reports[0]
            .external_model_definitions
            .iter()
            .all(|e| e["resolution"]["model"] == "shared/Model:shared")
    );
}
