use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use bishin_collect::{Module, ModuleGraph};
use bishin_jobs::Job;
use bishin_parser::Test;
use indoc::formatdoc;

/// Errors that can be encountered while generating test jobs.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// An error encountered while parsing tests.
    #[error(transparent)]
    Parse(#[from] bishin_parser::Error),
    /// An error encountered while writing a generated test script.
    #[error("failed to write generated test script")]
    Write(#[source] std::io::Error),
}

/// The module path of a module and the parsed tests that it contained.
#[derive(Debug)]
struct ModuleTests {
    /// The module path of this set of tests.
    module_path: Vec<String>,
    /// The parsed tests that this module contained.
    tests: Vec<Test>,
}

/// Parse the tests associated with a module.
fn load_module_tests(module: &Module) -> Result<ModuleTests, Error> {
    let file_path = module.file_path();
    debug_assert!(file_path.is_some());
    let file_path = file_path.unwrap();
    let module_path = module.module_path_components();
    let parsed = bishin_parser::parse_test_file(&file_path)?;
    Ok::<_, Error>(ModuleTests {
        module_path,
        tests: parsed,
    })
}

/// Returns the filename of a generated test file.
///
/// This is computed by joining each component of the module path with `_`
/// characters and prepending the result with `test_`.
fn module_test_file_name(module_path: &[String]) -> String {
    format!("test_{}.sh", module_path.join("_"))
}

/// Performs transformations on the test body to generate a shell script.
fn transform_body(body: &str) -> String {
    formatdoc! {"
        #!/usr/bin/env bash

        {body}
    "}
}

/// Data about an individual test.
#[derive(Debug)]
struct TestJob {
    /// The name of the test itself.
    _name: String,
    /// The module path of the test, including the test name as
    /// the final component.
    module_path: Vec<String>,
    /// The filesystem path where the generated script will be written.
    script_path: PathBuf,
    /// The contents of the generated script.
    script_contents: String,
}

impl From<TestJob> for Job {
    fn from(test_job: TestJob) -> Self {
        Job {
            name: test_job.module_path.join("_"),
            args: vec![
                "bash".to_string(),
                test_job.script_path.to_string_lossy().to_string(),
            ],
            envs: HashMap::new(),
        }
    }
}

/// Generates the test-specific job information for each test in a module.
fn test_jobs_for_module(out_dir: &Path, module_tests: &ModuleTests) -> Vec<TestJob> {
    let mut test_jobs = Vec::new();
    for test in module_tests.tests.iter() {
        let mut full_path = module_tests.module_path.clone();
        full_path.push(test.name.clone());
        let filename = module_test_file_name(&full_path);
        let script_contents = transform_body(&test.body);
        let file_path = out_dir.join(filename);
        let test_job = TestJob {
            _name: test.name.clone(),
            module_path: full_path,
            script_path: file_path,
            script_contents,
        };
        test_jobs.push(test_job);
    }
    test_jobs
}

/// Generates all of the test-specific job information from a module graph.
fn make_test_jobs(out_dir: impl AsRef<Path>, modules: &ModuleGraph) -> Result<Vec<TestJob>, Error> {
    let out_dir = out_dir.as_ref();
    let tests_by_module = modules
        .iter_leaf_modules()
        .map(load_module_tests)
        .collect::<Result<Vec<ModuleTests>, _>>()?;
    let mut test_jobs = Vec::new();
    for module_tests in tests_by_module {
        let jobs = test_jobs_for_module(out_dir, &module_tests);
        test_jobs.extend(jobs);
    }
    Ok(test_jobs)
}

/// Writes the generated test scripts to disk.
fn write_test_scripts(test_jobs: &[TestJob]) -> Result<(), Error> {
    for job in test_jobs.iter() {
        std::fs::write(&job.script_path, &job.script_contents).map_err(Error::Write)?;
    }
    Ok(())
}

/// Generate a list of jobs from the graph of test modules.
pub fn generate_test_jobs(
    out_dir: impl AsRef<Path>,
    module_graph: &ModuleGraph,
) -> Result<Vec<Job>, Error> {
    let test_jobs = make_test_jobs(&out_dir, module_graph)?;
    write_test_scripts(&test_jobs)?;
    let jobs = test_jobs
        .into_iter()
        .map(|tj| tj.into())
        .collect::<Vec<_>>();
    Ok(jobs)
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    #[test]
    fn extracts_all_tests() {
        // The double braces are to escape the format-string syntax.
        let contents = formatdoc! {r#"
            @test foo {{
                echo "hello from foo"
            }}

            @test bar {{
                echo "hello from foo"
            }}
        "#};
        let tempdir = TempDir::new().unwrap();
        let module_file_path = tempdir.path().join("mymodule.b");
        std::fs::write(&module_file_path, contents).unwrap();
        let module = Module {
            module_path: vec!["mymodule".to_string()],
            file: Some(module_file_path),
        };
        let module_tests = load_module_tests(&module).unwrap();
        assert_eq!(module_tests.module_path, vec!["mymodule".to_string()]);
        assert_eq!(module_tests.tests[0].name, "foo".to_string());
        assert_eq!(module_tests.tests[1].name, "bar".to_string());
    }
}
