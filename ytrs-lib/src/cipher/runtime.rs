//! JavaScript runtime using deno_core for executing cipher functions.

use crate::{Error, Result};
use deno_core::{JsRuntime, RuntimeOptions};

/// A JavaScript executor backed by deno_core (V8).
pub struct JsExecutor {
    runtime: JsRuntime,
}

impl JsExecutor {
    /// Create a new JavaScript executor.
    pub fn new() -> Result<Self> {
        let runtime = JsRuntime::new(RuntimeOptions::default());
        Ok(Self { runtime })
    }

    /// Execute JavaScript code and return the result as a string.
    pub fn execute(&mut self, code: &str) -> Result<String> {
        let value_global = self
            .runtime
            .execute_script("<cipher>", code.to_string())
            .map_err(|e| Error::Cipher(format!("JS execution failed: {}", e)))?;

        // Convert the result to a string using deno_core's scope macro
        let result = {
            deno_core::scope!(scope, self.runtime);
            let value = value_global.open(scope);

            if value.is_string() {
                value
                    .to_string(scope)
                    .map(|s| s.to_rust_string_lossy(scope))
                    .unwrap_or_default()
            } else if value.is_undefined() || value.is_null() {
                String::new()
            } else {
                // Try to convert to string
                value
                    .to_string(scope)
                    .map(|s| s.to_rust_string_lossy(scope))
                    .unwrap_or_default()
            }
        };

        Ok(result)
    }

    /// Execute JavaScript code without expecting a return value.
    pub fn execute_void(&mut self, code: &str) -> Result<()> {
        self.runtime
            .execute_script("<cipher>", code.to_string())
            .map_err(|e| Error::Cipher(format!("JS execution failed: {}", e)))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_execution() {
        let mut executor = JsExecutor::new().unwrap();
        let result = executor.execute("1 + 1").unwrap();
        assert_eq!(result, "2");
    }

    #[test]
    fn test_string_manipulation() {
        let mut executor = JsExecutor::new().unwrap();
        let result = executor
            .execute(r#""hello".split("").reverse().join("")"#)
            .unwrap();
        assert_eq!(result, "olleh");
    }

    #[test]
    fn test_function_definition_and_call() {
        let mut executor = JsExecutor::new().unwrap();
        executor
            .execute_void("function reverse(s) { return s.split('').reverse().join(''); }")
            .unwrap();
        let result = executor.execute("reverse('hello')").unwrap();
        assert_eq!(result, "olleh");
    }
}
