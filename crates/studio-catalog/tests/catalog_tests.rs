use std::fs;
use std::path::Path;

use studio_catalog::*;
use tempfile::tempdir;

#[test]
fn test_frontmatter_parsing() {
    let raw = r#"---
name: sample-skill
description: >
  Folded description line 1
  line 2 of description.
license: MIT
---

# Title
Body content
"#;
    let fm = frontmatter(raw);
    assert_eq!(fm.get("name").map(String::as_str), Some("sample-skill"));
    assert_eq!(
        fm.get("description").map(String::as_str),
        Some("Folded description line 1 line 2 of description.")
    );
    assert_eq!(fm.get("license").map(String::as_str), Some("MIT"));

    let raw_pipe = r#"---
name: pipe-skill
description: |-
  First line
  Second line
---
"#;
    let fm_pipe = frontmatter(raw_pipe);
    assert_eq!(fm_pipe.get("name").map(String::as_str), Some("pipe-skill"));
    assert_eq!(
        fm_pipe.get("description").map(String::as_str),
        Some("First line Second line")
    );
}

#[test]
fn test_denied_commands() {
    let cmds = denied_commands();
    assert!(cmds.contains(&"npx skills add".to_string()));
    assert!(cmds.contains(&"npx skills install".to_string()));
}

#[test]
fn test_suggest_kits() {
    let yaml = r#"
schema: catalog@1
kits:
  motion-graphic:
    title: Motion graphic ngắn
    keywords: [motion, animation, chuyển động]
    use: {}
  shorts-reels:
    title: Shorts / Reels có giọng đọc và phụ đề
    keywords: [shorts, tiktok, phụ đề, lồng tiếng]
    use: {}
"#;
    let cat = Catalog::from_yaml(yaml).expect("parse catalog");

    let suggestions = suggest_kits(&cat, "tôi muốn làm video Chuyển Động cho sản phẩm");
    assert!(!suggestions.is_empty());
    assert_eq!(suggestions[0].0, "motion-graphic");

    let suggestions_reels = suggest_kits(&cat, "tạo shorts tiktok có phụ đề tự động");
    assert!(!suggestions_reels.is_empty());
    assert_eq!(suggestions_reels[0].0, "shorts-reels");
}

#[test]
fn test_synthetic_inventory_and_check() {
    let dir = tempdir().expect("tempdir");
    let sources_dir = dir.path().join("sources");
    let repo_dir = sources_dir.join("test__skills");
    let skill1_dir = repo_dir.join("skills").join("skill-one");
    let skill2_dir = repo_dir.join("skills").join("skill-two");

    fs::create_dir_all(&skill1_dir).unwrap();
    fs::create_dir_all(&skill2_dir).unwrap();
    fs::create_dir_all(repo_dir.join(".git")).unwrap();

    fs::write(
        skill1_dir.join("SKILL.md"),
        "---\nname: skill-one\ndescription: Skill one description for testing.\n---\n# Skill One\n",
    )
    .unwrap();

    fs::write(
        skill2_dir.join("SKILL.md"),
        "---\nname: skill-two\ndescription: Skill two description.\n---\n# Skill Two\n",
    )
    .unwrap();

    let inv = Inventory::scan(&sources_dir).expect("scan inventory");
    assert_eq!(inv.repositories.len(), 1);
    assert_eq!(inv.repositories[0].repo, "test/skills");
    assert_eq!(inv.repositories[0].skills.len(), 2);

    // Build matching catalog
    let catalog_yaml = r#"
schema: catalog@1
sources:
  test/skills:
    commit: ""
modules:
  test.module:
    skills:
      test/skills: [skill-one, skill-two]
kits:
  test-kit:
    title: Test Kit
    use:
      test.module: all
"#;
    let catalog = Catalog::from_yaml(catalog_yaml).expect("parse catalog");
    let rep = check(&catalog, &inv);
    assert!(
        rep.is_ok(),
        "Expected check to pass, problems: {:?}",
        rep.problems
    );
    assert_eq!(rep.kit_reports.len(), 1);
    assert_eq!(rep.kit_reports[0].loaded, 2);
}

#[test]
fn test_resolve_with_requires_and_skill_requires() {
    let yaml = r#"
schema: catalog@1
modules:
  core.always:
    always: true
    skills:
      test/core: [core-skill]
  dep.module:
    skills:
      test/dep: [dep-skill]
  apple.module:
    skills:
      test/apple: [apple-skill]
  playbook.main:
    requires: [dep.module]
    skills:
      test/main: [main-one, main-two]
    skill_requires:
      main-two: [apple.module/apple-skill]
kits:
  main-kit:
    title: Main Kit
    use:
      playbook.main: [main-two]
"#;
    let catalog = Catalog::from_yaml(yaml).expect("parse catalog");
    let active = resolve(&catalog, Some("main-kit"), &[]).expect("resolve");

    let skill_names: Vec<&str> = active.skills.iter().map(|s| s.name.as_str()).collect();
    // core.always (always) -> core-skill
    assert!(skill_names.contains(&"core-skill"));
    // playbook.main (from kit) -> main-two
    assert!(skill_names.contains(&"main-two"));
    // requires -> dep.module -> dep-skill
    assert!(skill_names.contains(&"dep-skill"));
    // skill_requires from main-two -> apple.module/apple-skill -> apple-skill
    assert!(skill_names.contains(&"apple-skill"));
    // main-one was NOT selected in the kit
    assert!(!skill_names.contains(&"main-one"));
}

#[test]
fn test_materialize_plugin_layout() {
    let dir = tempdir().expect("tempdir");
    let sources_dir = dir.path().join("sources");
    let repo_dir = sources_dir.join("test__skills");
    let skill1_dir = repo_dir.join("skills").join("skill-one");
    let skill2_dir = repo_dir.join("skills").join("skill-two");
    let adapt_skill_dir = repo_dir.join("skills").join("adapt-me");

    fs::create_dir_all(&skill1_dir).unwrap();
    fs::create_dir_all(&skill2_dir).unwrap();
    fs::create_dir_all(&adapt_skill_dir).unwrap();
    fs::create_dir_all(repo_dir.join(".git")).unwrap();

    fs::write(
        skill1_dir.join("SKILL.md"),
        "---\nname: skill-one\ndescription: Skill one\n---\n# Original 1\n",
    )
    .unwrap();
    fs::write(
        skill2_dir.join("SKILL.md"),
        "---\nname: skill-two\ndescription: Skill two\n---\n# Original 2\n",
    )
    .unwrap();
    fs::write(
        adapt_skill_dir.join("SKILL.md"),
        "---\nname: adapt-me\ndescription: Adapt skill\n---\n# Original Adapt\n",
    )
    .unwrap();

    let yaml = r#"
schema: catalog@1
adapt:
  test/skills/adapt-me: "Use Studio MCP for this."
modules:
  router.mod:
    router: generated
    skills:
      test/skills: [skill-one, skill-two]
  adapt.mod:
    skills:
      test/skills: [adapt-me]
kits:
  my-kit:
    title: My Kit
    use:
      router.mod: all
      adapt.mod: all
"#;
    let catalog = Catalog::from_yaml(yaml).expect("parse");
    let active = resolve(&catalog, Some("my-kit"), &[]).expect("resolve");

    let out_dir = dir.path().join("plugin_out");
    materialize_plugin(&active, &catalog, &sources_dir, &out_dir).expect("materialize");

    // Check .claude-plugin/plugin.json
    let plugin_json_path = out_dir.join(".claude-plugin").join("plugin.json");
    assert!(plugin_json_path.exists());
    let plugin_json = fs::read_to_string(plugin_json_path).unwrap();
    assert!(plugin_json.contains("studio-workspace"));

    // Check generated router
    let router_dir = out_dir.join("skills").join("router-mod");
    assert!(router_dir.exists());
    let router_md = router_dir.join("SKILL.md");
    assert!(router_md.exists());
    let router_text = fs::read_to_string(router_md).unwrap();
    assert!(router_text.contains("Studio router for module router.mod"));

    // Check refs sub-skills
    let ref1 = router_dir.join("refs").join("skill-one").join("GUIDE.md");
    let ref2 = router_dir.join("refs").join("skill-two").join("GUIDE.md");
    assert!(ref1.exists(), "GUIDE.md for skill-one should exist");
    assert!(ref2.exists(), "GUIDE.md for skill-two should exist");
    // Ensure original SKILL.md is not in refs
    assert!(!router_dir
        .join("refs")
        .join("skill-one")
        .join("SKILL.md")
        .exists());

    // Check adapted skill wrapper
    let adapted_dir = out_dir.join("skills").join("adapt-me");
    let adapted_md = adapted_dir.join("SKILL.md");
    assert!(adapted_md.exists());
    let adapted_text = fs::read_to_string(adapted_md).unwrap();
    assert!(adapted_text.contains("Studio Adaptation"));
    assert!(adapted_text.contains("Use Studio MCP for this."));
    assert!(adapted_text.contains("Original Adapt"));
}

#[test]
fn test_real_catalog_yaml_parses_and_kits_valid() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let catalog_path = repo_root.join("catalog").join("catalog.yaml");
    assert!(
        catalog_path.exists(),
        "catalog.yaml should exist at repo root"
    );

    let catalog = Catalog::load_file(&catalog_path).expect("Failed to parse catalog.yaml");

    assert!(!catalog.sources.is_empty());
    assert!(!catalog.modules.is_empty());
    assert!(!catalog.kits.is_empty());

    // Verify every kit references existing modules
    for (kid, kit) in &catalog.kits {
        for (mid, sel) in &kit.use_modules {
            assert!(
                catalog.modules.contains_key(mid),
                "Kit {} references unknown module {}",
                kid,
                mid
            );
            if let Some(skill_names) = sel.as_slice() {
                let mod_def = &catalog.modules[mid];
                let mod_skill_names: Vec<String> = mod_def
                    .skills
                    .values()
                    .flat_map(|v| v.iter().cloned())
                    .collect();
                for s in skill_names {
                    assert!(
                        mod_skill_names.contains(s),
                        "Kit {} uses skill {} not in module {}",
                        kid,
                        s,
                        mid
                    );
                }
            }
        }
    }
}

#[test]
#[ignore]
fn test_check_against_real_sources() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let catalog_path = repo_root.join("catalog").join("catalog.yaml");
    let catalog = Catalog::load_file(&catalog_path).expect("parse catalog.yaml");

    let real_sources = Path::new("C:/Users/Dang/AppData/Local/LocalContentStudio/catalog/sources");
    if !real_sources.exists() {
        return;
    }

    let inv = Inventory::scan(real_sources).expect("scan real sources");
    let rep = check(&catalog, &inv);

    assert!(
        rep.problems.is_empty(),
        "Expected 0 problems with real sources, found: {:?}",
        rep.problems
    );
}
