use crate::errors::{RefineryError, Result};
use toml_edit::{DocumentMut, Item, Table, Value, value};

/// Metadata extracted from Cargo.toml
#[derive(Debug, Default, Clone)]
pub struct CargoMetadata {
    pub name: String,
    pub description: String,
    pub authors: Vec<String>,
    pub license: String,
    pub repository: String,
}

/// Updates Cargo.toml content with required metadata for installers.
///
/// # Errors
/// Returns error if TOML parsing fails or required structures cannot be created.
pub fn update_cargo_toml_with_metadata(content: &str) -> Result<String> {
    let mut cargo_toml = content.parse::<DocumentMut>()?;
    let metadata = get_cargo_metadata(&cargo_toml);

    ensure_table_exists(&mut cargo_toml, "package")?;
    let package = cargo_toml["package"]
        .as_table_mut()
        .ok_or_else(|| RefineryError::Config("'package' is not a table".into()))?;

    ensure_sub_table_exists(package, "metadata")?;
    let metadata_table = package["metadata"]
        .as_table_mut()
        .ok_or_else(|| RefineryError::Config("'metadata' is not a table".into()))?;

    // Inject Debian metadata
    ensure_sub_table_exists(metadata_table, "deb")?;
    let deb = metadata_table["deb"]
        .as_table_mut()
        .ok_or_else(|| RefineryError::Config("'deb' is not a table".into()))?;
    deb["name"] = value(&metadata.name);

    // Inject RPM metadata
    ensure_sub_table_exists(metadata_table, "generate-rpm")?;
    let rpm = metadata_table["generate-rpm"]
        .as_table_mut()
        .ok_or_else(|| RefineryError::Config("'generate-rpm' is not a table".into()))?;

    rpm["name"] = value(&metadata.name);
    // Add default assets for RPM if not present
    if rpm.get("assets").is_none() {
        let mut assets = toml_edit::Array::new();
        let mut asset = toml_edit::InlineTable::new();
        asset.insert(
            "source",
            Value::from(format!("target/release/{}", metadata.name)),
        );
        asset.insert("dest", Value::from(format!("/usr/bin/{}", metadata.name)));
        asset.insert("mode", Value::from("755"));
        assets.push(asset);
        rpm["assets"] = value(assets);
    }

    Ok(cargo_toml.to_string())
}

/// Injects specific fields into the [package] section of Cargo.toml.
///
/// # Errors
/// Returns error if TOML parsing fails.
pub fn inject_cargo_fields(
    content: &str,
    authors: Option<Vec<String>>,
    license: Option<String>,
    description: Option<String>,
    repository: Option<String>,
) -> Result<String> {
    let mut cargo_toml = content.parse::<DocumentMut>()?;
    ensure_table_exists(&mut cargo_toml, "package")?;
    let package = cargo_toml["package"]
        .as_table_mut()
        .ok_or_else(|| RefineryError::Config("'package' is not a table".into()))?;

    if let Some(a) = authors {
        let mut array = toml_edit::Array::new();
        for author in a {
            array.push(author);
        }
        package["authors"] = value(array);
    }

    if let Some(l) = license {
        package["license"] = value(l);
    }

    if let Some(d) = description {
        package["description"] = value(d);
    }

    if let Some(r) = repository {
        package["repository"] = value(r);
    }

    Ok(cargo_toml.to_string())
}

#[must_use]
pub fn get_cargo_metadata(doc: &DocumentMut) -> CargoMetadata {
    let package = doc.get("package");
    CargoMetadata {
        name: package
            .and_then(|p| p.get("name"))
            .and_then(Item::as_str)
            .unwrap_or("project")
            .to_string(),
        description: package
            .and_then(|p| p.get("description"))
            .and_then(Item::as_str)
            .unwrap_or("")
            .to_string(),
        authors: package
            .and_then(|p| p.get("authors"))
            .and_then(Item::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str())
                    .map(ToString::to_string)
                    .collect()
            })
            .unwrap_or_default(),
        license: package
            .and_then(|p| p.get("license"))
            .and_then(Item::as_str)
            .unwrap_or("")
            .to_string(),
        repository: package
            .and_then(|p| p.get("repository"))
            .and_then(Item::as_str)
            .unwrap_or("")
            .to_string(),
    }
}

fn ensure_table_exists(doc: &mut DocumentMut, key: &str) -> Result<()> {
    if doc.get(key).is_none() {
        doc.insert(key, Item::Table(Table::new()));
    } else if !doc[key].is_table() {
        return Err(RefineryError::Config(format!(
            "Key '{key}' in Cargo.toml exists but is not a table"
        )));
    }
    Ok(())
}

fn ensure_sub_table_exists(table: &mut Table, key: &str) -> Result<()> {
    if table.get(key).is_none() {
        table.insert(key, Item::Table(Table::new()));
    } else if !table[key].is_table() {
        return Err(RefineryError::Config(format!(
            "Sub-key {key} in Cargo.toml exists but is not a table"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_update_cargo_toml_with_metadata() -> Result<()> {
        let content = r#"[package]
name = "test-project"
version = "0.1.0"
"#;
        let updated = update_cargo_toml_with_metadata(content)?;
        assert!(updated.contains(r"[package.metadata.deb]"));
        assert!(updated.contains(r"[package.metadata.generate-rpm]"));
        assert!(updated.contains(r#"name = "test-project""#));
        Ok(())
    }

    #[test]
    fn test_inject_cargo_fields() -> Result<()> {
        let content = r#"[package]
name = "test"
"#;
        let updated = inject_cargo_fields(
            content,
            Some(vec!["Me".into()]),
            Some("MIT".into()),
            Some("Desc".into()),
            Some("https://github.com/test/test".into()),
        )?;
        assert!(updated.contains(r#"authors = ["Me"]"#));
        assert!(updated.contains(r#"license = "MIT""#));
        assert!(updated.contains(r#"description = "Desc""#));
        assert!(updated.contains(r#"repository = "https://github.com/test/test""#));
        Ok(())
    }
}
