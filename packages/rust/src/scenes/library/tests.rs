#![allow(clippy::expect_used)]

use std::path::Path;

use super::*;

/// A library answer written for these tests. The parameters are ours.
const LIBRARY: &str = r#"{"message":"success","status":200,"data":{"categories":[
  {"categoryName":"Natural","scenes":[
    {"sceneName":"Dawn","lightEffects":[
      {"scenceName":"","sceneCode":3,"sceneType":0,"scenceParam":"","specialEffect":[]}]},
    {"sceneName":"Tide","lightEffects":[
      {"scenceName":"A","sceneCode":300,"sceneType":2,"scenceParam":"AQID","specialEffect":[
        {"scenceParam":"CQkJ","supportSku":["H0001"]}]},
      {"scenceName":"B","sceneCode":301,"sceneType":2,"scenceParam":"BAUG","specialEffect":[]}]}]},
  {"categoryName":"Party","scenes":[
    {"sceneName":"Flash","lightEffects":[
      {"scenceName":"","sceneCode":400,"sceneType":2,"scenceParam":"Bw==","specialEffect":[]}]}]},
  {"categoryName":"Show","scenes":[
    {"sceneName":"Flash","lightEffects":[
      {"scenceName":"","sceneCode":401,"sceneType":2,"scenceParam":"CA==","specialEffect":[]}]}]}
]}}"#;

fn library(sku: &str) -> Vec<Scene> {
    parse(sku, LIBRARY, Path::new("test")).expect("should parse")
}

#[test]
fn names_each_variant_with_its_letter() {
    let names: Vec<String> = library("H0000").into_iter().map(|s| s.name).collect();
    assert_eq!(names, ["Dawn", "Tide A", "Tide B", "Flash", "Flash"]);
}

#[test]
fn takes_the_parameters_that_name_the_model() {
    let param = |sku| library(sku).get(1).expect("Tide A").param.clone();
    assert_eq!(param("H0001"), [9, 9, 9]);
    assert_eq!(param("H0000"), [1, 2, 3]);
}

#[test]
fn a_stored_scene_has_no_parameters() {
    let scenes = library("H0000");
    let dawn = scenes.first().expect("Dawn");
    assert_eq!(dawn.param, Vec::<u8>::new());
    assert_eq!((dawn.code, dawn.scene_type), (3, 0));
}

#[test]
fn finds_by_code_by_full_name_and_by_shared_name() {
    let scenes = library("H0000");
    assert_eq!(find("H0000", &scenes, "301").expect("code").name, "Tide B");
    assert_eq!(find("H0000", &scenes, "tide a").expect("name").code, 300);
    assert_eq!(find("H0000", &scenes, "DAWN").expect("name").code, 3);
}

#[test]
fn refuses_a_name_that_two_scenes_share() {
    let scenes = library("H0000");
    let err = find("H0000", &scenes, "Tide").expect_err("two variants");
    assert_eq!(err.code(), "ambiguous_scene");
    let err = find("H0000", &scenes, "flash").expect_err("two categories");
    assert_eq!(err.code(), "ambiguous_scene");
}

#[test]
fn refuses_what_matches_nothing() {
    let err = find("H0000", &library("H0000"), "Dusk").expect_err("no match");
    assert_eq!(err.code(), "unknown_scene");
}

#[test]
fn refuses_parameters_that_are_not_base64() {
    let text = LIBRARY.replace("AQID", "@@");
    let err = parse("H0000", &text, Path::new("test")).expect_err("bad base64");
    assert_eq!(err.code(), "scene_list_unreadable");
}
