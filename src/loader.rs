//! Schema 加载器
//!
//! 支持从文件系统加载 `.pkt` 文件并递归处理 import 语句。
//! 适用于 packet-core 作为独立库使用时的文件加载场景。

use crate::ast::Schema;
use crate::error::{CoreError, Result};
use crate::parser::parse_schema;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

/// 从文件加载并解析 schema，递归处理 import 语句
///
/// # Arguments
///
/// * `path` - `.pkt` 文件路径
///
/// # Returns
///
/// 成功返回合并后的 `Schema`
///
/// # Errors
///
/// 如果文件不存在、解析失败或检测到循环导入则返回错误
pub fn load_schema(path: &Path) -> Result<Schema> {
    let mut visited = HashSet::new();
    load_schema_inner(path, &mut visited)
}

/// 内部递归加载函数
fn load_schema_inner(path: &Path, visited: &mut HashSet<PathBuf>) -> Result<Schema> {
    let canonical = path
        .canonicalize()
        .map_err(|e| CoreError::io("canonicalize", format!("{path:?}: {e}")))?;

    if !visited.insert(canonical.clone()) {
        return Err(CoreError::validation(
            "E101",
            format!("circular import detected: {path:?}"),
        ));
    }

    let source =
        fs::read_to_string(path).map_err(|e| CoreError::io("read", format!("{path:?}: {e}")))?;

    let mut schema = parse_schema(&source)?;

    // 获取父目录用于解析相对导入路径
    let parent_dir = path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf();

    // 处理 import 语句
    // 收集 import 路径到 vec 以避免借用冲突
    let import_paths: Vec<String> = schema.imports.iter().map(|imp| imp.path.clone()).collect();

    for import_path in &import_paths {
        let import_file = parent_dir.join(import_path);
        let imported = load_schema_inner(&import_file, visited)?;

        // 合并导入的 schema
        for struct_def in imported.structs {
            if schema.get_struct(&struct_def.name).is_some() {
                return Err(CoreError::validation(
                    "E101",
                    format!(
                        "duplicate struct '{}' from import '{}'",
                        struct_def.name, import_path
                    ),
                ));
            }
            schema.add_struct(struct_def);
        }

        // 合并文件属性
        for attr in imported.file_attributes {
            if !schema.file_attributes.contains(&attr) {
                schema.add_file_attribute(attr);
            }
        }
    }

    visited.remove(&canonical);
    Ok(schema)
}

/// 批量加载多个 schema 文件并合并
///
/// # Arguments
///
/// * `paths` - 多个 `.pkt` 文件路径
///
/// # Returns
///
/// 成功返回合并后的 `Schema`
///
/// # Errors
///
/// 如果任一文件加载失败则返回错误
pub fn load_schemas<P: AsRef<Path>>(paths: &[P]) -> Result<Schema> {
    let mut merged = Schema::new();

    for path in paths {
        let schema = load_schema(path.as_ref())?;
        for struct_def in schema.structs {
            if merged.get_struct(&struct_def.name).is_some() {
                return Err(CoreError::validation(
                    "E101",
                    format!("duplicate struct '{}' across schema files", struct_def.name),
                ));
            }
            merged.add_struct(struct_def);
        }
    }

    Ok(merged)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_load_simple_schema() {
        let mut file = NamedTempFile::new().unwrap();
        write!(
            file,
            r#"
            #[send]
            struct Test {{
                value: u32 = 42,
            }}
            "#
        )
        .unwrap();

        let schema = load_schema(file.path()).unwrap();
        assert!(schema.get_struct("Test").is_some());
    }

    #[test]
    fn test_load_nonexistent_file() {
        let result = load_schema(Path::new("nonexistent.pkt"));
        assert!(result.is_err());
    }

    #[test]
    fn test_load_invalid_schema() {
        let mut file = NamedTempFile::new().unwrap();
        write!(file, "invalid content @@@").unwrap();

        let result = load_schema(file.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_load_empty_schema() {
        let mut file = NamedTempFile::new().unwrap();
        write!(file, "").unwrap();

        let result = load_schema(file.path());
        assert!(result.is_ok());
    }
}
