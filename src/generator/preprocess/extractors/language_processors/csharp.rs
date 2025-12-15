use super::{Dependency, LanguageProcessor};
use crate::types::code::{InterfaceInfo, ParameterInfo};
use regex::Regex;
use std::path::Path;

#[derive(Debug)]
pub struct CSharpProcessor {
    using_regex: Regex,
    namespace_regex: Regex,
    method_regex: Regex,
    class_regex: Regex,
    interface_regex: Regex,
    enum_regex: Regex,
    struct_regex: Regex,
    property_regex: Regex,
    constructor_regex: Regex,
}

impl CSharpProcessor {
    pub fn new() -> Self {
        Self {
            using_regex: Regex::new(r"^\s*using\s+([^;]+);").unwrap(),
            namespace_regex: Regex::new(r"^\s*namespace\s+([^;\{]+)").unwrap(),
            method_regex: Regex::new(r"^\s*(public|private|protected|internal)?\s*(static)?\s*(virtual|override|abstract|sealed)?\s*(async)?\s*(\w+)\s+(\w+)\s*\(([^)]*)\)").unwrap(),
            class_regex: Regex::new(r"^\s*(public|private|protected|internal)?\s*(static)?\s*(abstract)?\s*(sealed)?\s*(partial)?\s*class\s+(\w+)").unwrap(),
            interface_regex: Regex::new(r"^\s*(public|private|protected|internal)?\s*(partial)?\s*interface\s+(\w+)").unwrap(),
            enum_regex: Regex::new(r"^\s*(public|private|protected|internal)?\s*enum\s+(\w+)").unwrap(),
            struct_regex: Regex::new(r"^\s*(public|private|protected|internal)?\s*(readonly)?\s*(partial)?\s*struct\s+(\w+)").unwrap(),
            property_regex: Regex::new(r"^\s*(public|private|protected|internal)?\s*(static)?\s*(virtual|override|abstract)?\s*(\w+)\s+(\w+)\s*\{\s*(get|set)").unwrap(),
            constructor_regex: Regex::new(r"^\s*(public|private|protected|internal)?\s*(\w+)\s*\(([^)]*)\)").unwrap(),
        }
    }
}

impl LanguageProcessor for CSharpProcessor {
    fn supported_extensions(&self) -> Vec<&'static str> {
        vec!["cs", "csproj", "sln", "sqlproj", "sql"]
    }
    
    fn extract_dependencies(&self, content: &str, file_path: &Path) -> Vec<Dependency> {
        let mut dependencies = Vec::new();
        let source_file = file_path.to_string_lossy().to_string();
        
        // Handle .csproj files
        if file_path.extension().and_then(|e| e.to_str()) == Some("csproj") {
            return self.extract_csproj_dependencies(content, &source_file);
        }
        
        // Handle .sqlproj files
        if file_path.extension().and_then(|e| e.to_str()) == Some("sqlproj") {
            return self.extract_sqlproj_dependencies(content, &source_file);
        }
        
        // Handle .sln files
        if file_path.extension().and_then(|e| e.to_str()) == Some("sln") {
            return self.extract_sln_dependencies(content, &source_file);
        }
        
        // Handle .sql files
        if file_path.extension().and_then(|e| e.to_str()) == Some("sql") {
            return self.extract_sql_dependencies(content, &source_file);
        }
        
        // Handle .cs files
        for (line_num, line) in content.lines().enumerate() {
            // Extract using statements
            if let Some(captures) = self.using_regex.captures(line) {
                if let Some(using_path) = captures.get(1) {
                    let using_str = using_path.as_str().trim();
                    
                    // Skip using static and using alias
                    if using_str.starts_with("static ") || using_str.contains(" = ") {
                        continue;
                    }
                    
                    let is_external = using_str.starts_with("System") || 
                                    using_str.starts_with("Microsoft") ||
                                    !using_str.contains(".");
                    
                    // Parse dependency name
                    let dependency_name = self.extract_dependency_name(using_str);
                    
                    dependencies.push(Dependency {
                        name: dependency_name,
                        path: Some(source_file.clone()),
                        is_external,
                        line_number: Some(line_num + 1),
                        dependency_type: "using".to_string(),
                        version: None,
                    });
                }
            }
            
            // Extract namespace statement
            if let Some(captures) = self.namespace_regex.captures(line) {
                if let Some(namespace_name) = captures.get(1) {
                    dependencies.push(Dependency {
                        name: namespace_name.as_str().trim().to_string(),
                        path: Some(source_file.clone()),
                        is_external: false,
                        line_number: Some(line_num + 1),
                        dependency_type: "namespace".to_string(),
                        version: None,
                    });
                }
            }
        }
        
        dependencies
    }
    
    fn determine_component_type(&self, file_path: &Path, content: &str) -> String {
        let file_name = file_path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");
        
        // Check for project files
        if file_name.ends_with(".csproj") {
            // Determine project type from SDK or OutputType
            if content.contains("Microsoft.NET.Sdk.Web") {
                return "csharp_web_project".to_string();
            } else if content.contains("<OutputType>Exe</OutputType>") {
                return "csharp_console_project".to_string();
            } else if content.contains("<OutputType>Library</OutputType>") || content.contains("Microsoft.NET.Sdk") {
                return "csharp_library_project".to_string();
            } else if content.contains("Microsoft.NET.Test.Sdk") || file_name.contains("Test") {
                return "csharp_test_project".to_string();
            }
            return "csharp_project".to_string();
        }
        
        // Check for SQL project files
        if file_name.ends_with(".sqlproj") {
            return "sql_database_project".to_string();
        }
        
        // Check for solution files
        if file_name.ends_with(".sln") {
            return "csharp_solution".to_string();
        }
        
        // Check for SQL files
        if file_name.ends_with(".sql") {
            if content.to_uppercase().contains("CREATE TABLE") || content.to_uppercase().contains("ALTER TABLE") {
                return "sql_table_definition".to_string();
            } else if content.to_uppercase().contains("CREATE PROCEDURE") || content.to_uppercase().contains("ALTER PROCEDURE") {
                return "sql_stored_procedure".to_string();
            } else if content.to_uppercase().contains("CREATE VIEW") || content.to_uppercase().contains("ALTER VIEW") {
                return "sql_view".to_string();
            } else if content.to_uppercase().contains("CREATE FUNCTION") || content.to_uppercase().contains("ALTER FUNCTION") {
                return "sql_function".to_string();
            } else if content.to_uppercase().contains("CREATE TRIGGER") {
                return "sql_trigger".to_string();
            }
            return "sql_script".to_string();
        }
        
        // Check for test files
        if file_name.ends_with("Test.cs") || file_name.ends_with("Tests.cs") ||
           content.contains("[Test]") || content.contains("[TestMethod]") {
            return "csharp_test".to_string();
        }
        
        // Check for common patterns
        if content.contains("interface ") {
            "csharp_interface".to_string()
        } else if content.contains("enum ") {
            "csharp_enum".to_string()
        } else if content.contains("struct ") {
            "csharp_struct".to_string()
        } else if content.contains("abstract class") {
            "csharp_abstract_class".to_string()
        } else if content.contains("static class") {
            "csharp_static_class".to_string()
        } else if content.contains("sealed class") {
            "csharp_sealed_class".to_string()
        } else if content.contains("partial class") {
            "csharp_partial_class".to_string()
        } else if content.contains("class ") {
            "csharp_class".to_string()
        } else {
            "csharp_file".to_string()
        }
    }
    
    fn is_important_line(&self, line: &str) -> bool {
        let trimmed = line.trim();
        
        // Type declarations
        if trimmed.starts_with("public class ") || trimmed.starts_with("class ") ||
           trimmed.starts_with("interface ") || trimmed.starts_with("enum ") ||
           trimmed.starts_with("struct ") || trimmed.starts_with("public ") || 
           trimmed.starts_with("private ") || trimmed.starts_with("protected ") ||
           trimmed.starts_with("internal ") || trimmed.starts_with("using ") ||
           trimmed.starts_with("namespace ") {
            return true;
        }
        
        // Attributes
        if trimmed.starts_with('[') && trimmed.contains(']') {
            return true;
        }
        
        // Important comments
        if trimmed.contains("TODO") || trimmed.contains("FIXME") || 
           trimmed.contains("NOTE") || trimmed.contains("HACK") {
            return true;
        }
        
        false
    }
    
    fn language_name(&self) -> &'static str {
        "C#"
    }

    fn extract_interfaces(&self, content: &str, _file_path: &Path) -> Vec<InterfaceInfo> {
        let mut interfaces = Vec::new();
        let lines: Vec<&str> = content.lines().collect();
        
        for (i, line) in lines.iter().enumerate() {
            // Extract class definitions
            if let Some(captures) = self.class_regex.captures(line) {
                let visibility = captures.get(1).map(|m| m.as_str()).unwrap_or("private");
                let is_static = captures.get(2).is_some();
                let is_abstract = captures.get(3).is_some();
                let is_sealed = captures.get(4).is_some();
                let is_partial = captures.get(5).is_some();
                let name = captures.get(6).map(|m| m.as_str()).unwrap_or("").to_string();
                
                let mut interface_type = "class".to_string();
                if is_static {
                    interface_type = "static_class".to_string();
                } else if is_abstract {
                    interface_type = "abstract_class".to_string();
                } else if is_sealed {
                    interface_type = "sealed_class".to_string();
                } else if is_partial {
                    interface_type = "partial_class".to_string();
                }
                
                interfaces.push(InterfaceInfo {
                    name,
                    interface_type,
                    visibility: visibility.to_string(),
                    parameters: Vec::new(),
                    return_type: None,
                    description: self.extract_xml_doc(&lines, i),
                });
            }
            
            // Extract interface definitions
            if let Some(captures) = self.interface_regex.captures(line) {
                let visibility = captures.get(1).map(|m| m.as_str()).unwrap_or("private");
                let is_partial = captures.get(2).is_some();
                let name = captures.get(3).map(|m| m.as_str()).unwrap_or("").to_string();
                
                let interface_type = if is_partial {
                    "partial_interface".to_string()
                } else {
                    "interface".to_string()
                };
                
                interfaces.push(InterfaceInfo {
                    name,
                    interface_type,
                    visibility: visibility.to_string(),
                    parameters: Vec::new(),
                    return_type: None,
                    description: self.extract_xml_doc(&lines, i),
                });
            }
            
            // Extract struct definitions
            if let Some(captures) = self.struct_regex.captures(line) {
                let visibility = captures.get(1).map(|m| m.as_str()).unwrap_or("private");
                let is_readonly = captures.get(2).is_some();
                let is_partial = captures.get(3).is_some();
                let name = captures.get(4).map(|m| m.as_str()).unwrap_or("").to_string();
                
                let mut interface_type = "struct".to_string();
                if is_readonly {
                    interface_type = "readonly_struct".to_string();
                } else if is_partial {
                    interface_type = "partial_struct".to_string();
                }
                
                interfaces.push(InterfaceInfo {
                    name,
                    interface_type,
                    visibility: visibility.to_string(),
                    parameters: Vec::new(),
                    return_type: None,
                    description: self.extract_xml_doc(&lines, i),
                });
            }
            
            // Extract enum definitions
            if let Some(captures) = self.enum_regex.captures(line) {
                let visibility = captures.get(1).map(|m| m.as_str()).unwrap_or("private");
                let name = captures.get(2).map(|m| m.as_str()).unwrap_or("").to_string();
                
                interfaces.push(InterfaceInfo {
                    name,
                    interface_type: "enum".to_string(),
                    visibility: visibility.to_string(),
                    parameters: Vec::new(),
                    return_type: None,
                    description: self.extract_xml_doc(&lines, i),
                });
            }
            
            // Extract property definitions
            if let Some(captures) = self.property_regex.captures(line) {
                let visibility = captures.get(1).map(|m| m.as_str()).unwrap_or("private");
                let is_static = captures.get(2).is_some();
                let modifier = captures.get(3).map(|m| m.as_str()).unwrap_or("");
                let return_type = captures.get(4).map(|m| m.as_str()).unwrap_or("").to_string();
                let name = captures.get(5).map(|m| m.as_str()).unwrap_or("").to_string();
                
                let mut interface_type = "property".to_string();
                if is_static {
                    interface_type = "static_property".to_string();
                } else if modifier == "virtual" {
                    interface_type = "virtual_property".to_string();
                } else if modifier == "override" {
                    interface_type = "override_property".to_string();
                } else if modifier == "abstract" {
                    interface_type = "abstract_property".to_string();
                }
                
                interfaces.push(InterfaceInfo {
                    name,
                    interface_type,
                    visibility: visibility.to_string(),
                    parameters: Vec::new(),
                    return_type: Some(return_type),
                    description: self.extract_xml_doc(&lines, i),
                });
            }
            
            // Extract method definitions
            if let Some(captures) = self.method_regex.captures(line) {
                let visibility = captures.get(1).map(|m| m.as_str()).unwrap_or("private");
                let is_static = captures.get(2).is_some();
                let modifier = captures.get(3).map(|m| m.as_str()).unwrap_or("");
                let is_async = captures.get(4).is_some();
                let return_type = captures.get(5).map(|m| m.as_str()).unwrap_or("").to_string();
                let name = captures.get(6).map(|m| m.as_str()).unwrap_or("").to_string();
                let params_str = captures.get(7).map(|m| m.as_str()).unwrap_or("");
                
                // Skip C# keywords
                if return_type == "if" || return_type == "for" || return_type == "while" || 
                   return_type == "foreach" || return_type == "switch" || return_type == "try" ||
                   return_type == "using" || return_type == "lock" {
                    continue;
                }
                
                let parameters = self.parse_csharp_parameters(params_str);
                let mut interface_type = "method".to_string();
                if is_static {
                    interface_type = "static_method".to_string();
                } else if is_async {
                    interface_type = "async_method".to_string();
                } else if modifier == "virtual" {
                    interface_type = "virtual_method".to_string();
                } else if modifier == "override" {
                    interface_type = "override_method".to_string();
                } else if modifier == "abstract" {
                    interface_type = "abstract_method".to_string();
                } else if modifier == "sealed" {
                    interface_type = "sealed_method".to_string();
                }
                
                interfaces.push(InterfaceInfo {
                    name,
                    interface_type,
                    visibility: visibility.to_string(),
                    parameters,
                    return_type: Some(return_type),
                    description: self.extract_xml_doc(&lines, i),
                });
            }
            
            // Extract constructors
            if let Some(captures) = self.constructor_regex.captures(line) {
                let visibility = captures.get(1).map(|m| m.as_str()).unwrap_or("private");
                let name = captures.get(2).map(|m| m.as_str()).unwrap_or("").to_string();
                let params_str = captures.get(3).map(|m| m.as_str()).unwrap_or("");
                
                // Simple check if it's a constructor (name starts with uppercase)
                if name.chars().next().map_or(false, |c| c.is_uppercase()) {
                    let parameters = self.parse_csharp_parameters(params_str);
                    
                    interfaces.push(InterfaceInfo {
                        name,
                        interface_type: "constructor".to_string(),
                        visibility: visibility.to_string(),
                        parameters,
                        return_type: None,
                        description: self.extract_xml_doc(&lines, i),
                    });
                }
            }
        }
        
        interfaces
    }
}

impl CSharpProcessor {
    /// Extract dependencies from .csproj files (NuGet packages and project references)
    fn extract_csproj_dependencies(&self, content: &str, source_file: &str) -> Vec<Dependency> {
        let mut dependencies = Vec::new();
        
        for (line_num, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            
            // Extract NuGet package references: <PackageReference Include="Package.Name" Version="1.0.0" />
            if trimmed.starts_with("<PackageReference") && trimmed.contains("Include=") {
                if let Some(start) = trimmed.find("Include=\"") {
                    let after_include = &trimmed[start + 9..];
                    if let Some(end) = after_include.find('"') {
                        let package_name = &after_include[..end];
                        
                        // Extract version if present
                        let version = if let Some(ver_start) = trimmed.find("Version=\"") {
                            let after_version = &trimmed[ver_start + 9..];
                            after_version.find('"').map(|ver_end| after_version[..ver_end].to_string())
                        } else {
                            None
                        };
                        
                        dependencies.push(Dependency {
                            name: package_name.to_string(),
                            path: Some(source_file.to_string()),
                            is_external: true,
                            line_number: Some(line_num + 1),
                            dependency_type: "nuget_package".to_string(),
                            version,
                        });
                    }
                }
            }
            
            // Extract project references: <ProjectReference Include="..\Other.Project\Other.Project.csproj" />
            if trimmed.starts_with("<ProjectReference") && trimmed.contains("Include=") {
                if let Some(start) = trimmed.find("Include=\"") {
                    let after_include = &trimmed[start + 9..];
                    if let Some(end) = after_include.find('"') {
                        let project_path = &after_include[..end];
                        
                        // Extract project name from path
                        let project_name = project_path
                            .split(['/', '\\'])
                            .last()
                            .unwrap_or(project_path)
                            .trim_end_matches(".csproj")
                            .to_string();
                        
                        dependencies.push(Dependency {
                            name: project_name,
                            path: Some(source_file.to_string()),
                            is_external: false,
                            line_number: Some(line_num + 1),
                            dependency_type: "project_reference".to_string(),
                            version: None,
                        });
                    }
                }
            }
            
            // Extract framework references: <FrameworkReference Include="Microsoft.AspNetCore.App" />
            if trimmed.starts_with("<FrameworkReference") && trimmed.contains("Include=") {
                if let Some(start) = trimmed.find("Include=\"") {
                    let after_include = &trimmed[start + 9..];
                    if let Some(end) = after_include.find('"') {
                        let framework_name = &after_include[..end];
                        
                        dependencies.push(Dependency {
                            name: framework_name.to_string(),
                            path: Some(source_file.to_string()),
                            is_external: true,
                            line_number: Some(line_num + 1),
                            dependency_type: "framework_reference".to_string(),
                            version: None,
                        });
                    }
                }
            }
        }
        
        dependencies
    }
    
    /// Extract project references from .sln files
    fn extract_sln_dependencies(&self, content: &str, source_file: &str) -> Vec<Dependency> {
        let mut dependencies = Vec::new();
        
        for (line_num, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            
            // Extract project entries: Project("{FAE04EC0-301F-11D3-BF4B-00C04F79EFBC}") = "ProjectName", "Path\ProjectName.csproj", "{GUID}"
            if trimmed.starts_with("Project(") && trimmed.contains(".csproj") {
                // Extract project name (between first pair of quotes after =)
                if let Some(name_start) = trimmed.find("= \"") {
                    let after_equals = &trimmed[name_start + 3..];
                    if let Some(name_end) = after_equals.find('"') {
                        let project_name = &after_equals[..name_end];
                        
                        dependencies.push(Dependency {
                            name: project_name.to_string(),
                            path: Some(source_file.to_string()),
                            is_external: false,
                            line_number: Some(line_num + 1),
                            dependency_type: "solution_project".to_string(),
                            version: None,
                        });
                    }
                }
            }
        }
        
        dependencies
    }

    /// Parse C# method parameters
    fn parse_csharp_parameters(&self, params_str: &str) -> Vec<ParameterInfo> {
        let mut parameters = Vec::new();
        
        if params_str.trim().is_empty() {
            return parameters;
        }
        
        // Simple parameter parsing, handling basic cases
        for param in params_str.split(',') {
            let param = param.trim();
            if param.is_empty() {
                continue;
            }
            
            // Parse parameter format: Type name, ref Type name, out Type name, params Type[] name, Type name = default
            let parts: Vec<&str> = param.split_whitespace().collect();
            if parts.len() >= 2 {
                let (param_type, name, is_optional) = if parts[0] == "ref" || parts[0] == "out" || parts[0] == "in" || parts[0] == "params" {
                    if parts.len() >= 3 {
                        (parts[1].to_string(), parts[2].to_string(), false)
                    } else {
                        continue;
                    }
                } else {
                    // Check for default value (optional parameter)
                    let has_default = param.contains('=');
                    let name = parts[1].split('=').next().unwrap_or(parts[1]).to_string();
                    (parts[0].to_string(), name, has_default)
                };
                
                // Handle generic types and nullable types
                let clean_type = if param_type.contains('<') || param_type.contains('?') {
                    param_type
                } else {
                    param_type
                };
                
                parameters.push(ParameterInfo {
                    name,
                    param_type: clean_type,
                    is_optional,
                    description: None,
                });
            }
        }
        
        parameters
    }
    
    /// Extract XML documentation comments
    fn extract_xml_doc(&self, lines: &[&str], current_line: usize) -> Option<String> {
        let mut doc_lines = Vec::new();
        
        // Search upward for XML doc comments
        for i in (0..current_line).rev() {
            let line = lines[i].trim();
            
            if line.starts_with("///") {
                let content = line.trim_start_matches("///").trim();
                // Extract content from <summary> tags
                if content.starts_with("<summary>") {
                    let text = content.trim_start_matches("<summary>").trim_end_matches("</summary>").trim();
                    if !text.is_empty() {
                        doc_lines.insert(0, text.to_string());
                    }
                } else if content.ends_with("</summary>") {
                    let text = content.trim_end_matches("</summary>").trim();
                    if !text.is_empty() {
                        doc_lines.insert(0, text.to_string());
                    }
                } else if !content.is_empty() && !content.starts_with('<') && !content.ends_with('>') {
                    doc_lines.insert(0, content.to_string());
                }
            } else if !line.is_empty() && !line.starts_with('[') {
                break;
            }
        }
        
        if doc_lines.is_empty() {
            None
        } else {
            Some(doc_lines.join(" "))
        }
    }

    /// Extract dependency name from C# using path
    fn extract_dependency_name(&self, using_path: &str) -> String {
        // For System.Collections.Generic, return Generic
        if let Some(namespace_name) = using_path.split('.').last() {
            namespace_name.to_string()
        } else {
            using_path.to_string()
        }
    }
    
    /// Extract dependencies from .sqlproj files (SQL project references and build items)
    fn extract_sqlproj_dependencies(&self, content: &str, source_file: &str) -> Vec<Dependency> {
        let mut dependencies = Vec::new();
        
        for (line_num, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            
            // Extract SQL file references: <Build Include="dbo\Tables\Users.sql" />
            if (trimmed.starts_with("<Build") || trimmed.starts_with("<PreDeploy") || 
                trimmed.starts_with("<PostDeploy")) && trimmed.contains("Include=") {
                if let Some(start) = trimmed.find("Include=\"") {
                    let after_include = &trimmed[start + 9..];
                    if let Some(end) = after_include.find('"') {
                        let file_path = &after_include[..end];
                        
                        // Extract SQL object name and type from path
                        let parts: Vec<&str> = file_path.split(['/', '\\', '.']).collect();
                        let object_type = if parts.len() > 2 {
                            parts[parts.len() - 3].to_string() // e.g., "Tables", "StoredProcedures"
                        } else {
                            "sql_object".to_string()
                        };
                        
                        let object_name = parts
                            .iter()
                            .rev()
                            .nth(1)
                            .unwrap_or(&"unknown")
                            .to_string();
                        
                        dependencies.push(Dependency {
                            name: object_name,
                            path: Some(source_file.to_string()),
                            is_external: false,
                            line_number: Some(line_num + 1),
                            dependency_type: object_type,
                            version: None,
                        });
                    }
                }
            }
            
            // Extract project references: <ProjectReference Include="..\OtherDatabase\OtherDatabase.sqlproj" />
            if trimmed.starts_with("<ProjectReference") && trimmed.contains("Include=") {
                if let Some(start) = trimmed.find("Include=\"") {
                    let after_include = &trimmed[start + 9..];
                    if let Some(end) = after_include.find('"') {
                        let project_path = &after_include[..end];
                        
                        // Extract project name from path
                        let project_name = project_path
                            .split(['/', '\\'])
                            .last()
                            .unwrap_or(project_path)
                            .trim_end_matches(".sqlproj")
                            .to_string();
                        
                        dependencies.push(Dependency {
                            name: project_name,
                            path: Some(source_file.to_string()),
                            is_external: false,
                            line_number: Some(line_num + 1),
                            dependency_type: "database_reference".to_string(),
                            version: None,
                        });
                    }
                }
            }
            
            // Extract DACPAC references: <ArtifactReference Include="..\..\Packages\DatabaseName.dacpac" />
            if trimmed.starts_with("<ArtifactReference") && trimmed.contains("Include=") {
                if let Some(start) = trimmed.find("Include=\"") {
                    let after_include = &trimmed[start + 9..];
                    if let Some(end) = after_include.find('"') {
                        let dacpac_path = &after_include[..end];
                        
                        let dacpac_name = dacpac_path
                            .split(['/', '\\'])
                            .last()
                            .unwrap_or(dacpac_path)
                            .trim_end_matches(".dacpac")
                            .to_string();
                        
                        dependencies.push(Dependency {
                            name: dacpac_name,
                            path: Some(source_file.to_string()),
                            is_external: true,
                            line_number: Some(line_num + 1),
                            dependency_type: "dacpac_reference".to_string(),
                            version: None,
                        });
                    }
                }
            }
        }
        
        dependencies
    }
    
    /// Extract dependencies from .sql files (table references, stored procedure calls, etc.)
    fn extract_sql_dependencies(&self, content: &str, source_file: &str) -> Vec<Dependency> {
        let mut dependencies = Vec::new();
        
        for (line_num, line) in content.lines().enumerate() {
            let upper_line = line.to_uppercase();
            let trimmed = line.trim();
            
            // Skip comments
            if trimmed.starts_with("--") || trimmed.starts_with("/*") {
                continue;
            }
            
            // Extract table references from FROM clause
            if upper_line.contains(" FROM ") {
                if let Some(from_pos) = upper_line.find(" FROM ") {
                    let after_from = &line[from_pos + 6..];
                    let table_part = after_from
                        .split_whitespace()
                        .next()
                        .unwrap_or("")
                        .trim_matches(|c: char| !c.is_alphanumeric() && c != '.' && c != '_' && c != '[' && c != ']');
                    
                    if !table_part.is_empty() {
                        dependencies.push(Dependency {
                            name: table_part.to_string(),
                            path: Some(source_file.to_string()),
                            is_external: false,
                            line_number: Some(line_num + 1),
                            dependency_type: "table_reference".to_string(),
                            version: None,
                        });
                    }
                }
            }
            
            // Extract table references from JOIN clause
            if upper_line.contains(" JOIN ") {
                if let Some(join_pos) = upper_line.find(" JOIN ") {
                    let after_join = &line[join_pos + 6..];
                    let table_part = after_join
                        .split_whitespace()
                        .next()
                        .unwrap_or("")
                        .trim_matches(|c: char| !c.is_alphanumeric() && c != '.' && c != '_' && c != '[' && c != ']');
                    
                    if !table_part.is_empty() {
                        dependencies.push(Dependency {
                            name: table_part.to_string(),
                            path: Some(source_file.to_string()),
                            is_external: false,
                            line_number: Some(line_num + 1),
                            dependency_type: "table_reference".to_string(),
                            version: None,
                        });
                    }
                }
            }
            
            // Extract table references from INSERT INTO
            if upper_line.contains("INSERT INTO ") {
                if let Some(insert_pos) = upper_line.find("INSERT INTO ") {
                    let after_insert = &line[insert_pos + 12..];
                    let table_part = after_insert
                        .split_whitespace()
                        .next()
                        .unwrap_or("")
                        .trim_matches(|c: char| !c.is_alphanumeric() && c != '.' && c != '_' && c != '[' && c != ']');
                    
                    if !table_part.is_empty() {
                        dependencies.push(Dependency {
                            name: table_part.to_string(),
                            path: Some(source_file.to_string()),
                            is_external: false,
                            line_number: Some(line_num + 1),
                            dependency_type: "table_reference".to_string(),
                            version: None,
                        });
                    }
                }
            }
            
            // Extract table references from UPDATE
            if upper_line.contains("UPDATE ") && !upper_line.contains("UPDATE STATISTICS") {
                if let Some(update_pos) = upper_line.find("UPDATE ") {
                    let after_update = &line[update_pos + 7..];
                    let table_part = after_update
                        .split_whitespace()
                        .next()
                        .unwrap_or("")
                        .trim_matches(|c: char| !c.is_alphanumeric() && c != '.' && c != '_' && c != '[' && c != ']');
                    
                    if !table_part.is_empty() {
                        dependencies.push(Dependency {
                            name: table_part.to_string(),
                            path: Some(source_file.to_string()),
                            is_external: false,
                            line_number: Some(line_num + 1),
                            dependency_type: "table_reference".to_string(),
                            version: None,
                        });
                    }
                }
            }
            
            // Extract table references from DELETE FROM
            if upper_line.contains("DELETE FROM ") {
                if let Some(delete_pos) = upper_line.find("DELETE FROM ") {
                    let after_delete = &line[delete_pos + 12..];
                    let table_part = after_delete
                        .split_whitespace()
                        .next()
                        .unwrap_or("")
                        .trim_matches(|c: char| !c.is_alphanumeric() && c != '.' && c != '_' && c != '[' && c != ']');
                    
                    if !table_part.is_empty() {
                        dependencies.push(Dependency {
                            name: table_part.to_string(),
                            path: Some(source_file.to_string()),
                            is_external: false,
                            line_number: Some(line_num + 1),
                            dependency_type: "table_reference".to_string(),
                            version: None,
                        });
                    }
                }
            }
            
            // Extract stored procedure calls: EXEC/EXECUTE ProcedureName
            if upper_line.contains("EXEC ") || upper_line.contains("EXECUTE ") {
                let exec_pos = if let Some(pos) = upper_line.find("EXECUTE ") {
                    pos + 8
                } else if let Some(pos) = upper_line.find("EXEC ") {
                    pos + 5
                } else {
                    continue;
                };
                
                let after_exec = &line[exec_pos..];
                let proc_name = after_exec
                    .split_whitespace()
                    .next()
                    .unwrap_or("")
                    .trim_matches(|c: char| !c.is_alphanumeric() && c != '.' && c != '_' && c != '[' && c != ']');
                
                if !proc_name.is_empty() && !proc_name.starts_with('@') {
                    dependencies.push(Dependency {
                        name: proc_name.to_string(),
                        path: Some(source_file.to_string()),
                        is_external: false,
                        line_number: Some(line_num + 1),
                        dependency_type: "stored_procedure_call".to_string(),
                        version: None,
                    });
                }
            }
        }
        
        dependencies
    }
}
