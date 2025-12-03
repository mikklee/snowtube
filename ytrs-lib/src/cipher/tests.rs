//! Integration tests for the cipher module.

#[cfg(test)]
mod tests {
    /// Test extracting cipher functions from a real YouTube player.js
    /// This test requires network access.
    #[tokio::test]
    #[ignore] // Run with: cargo test -- --ignored
    async fn test_extract_from_real_player() {
        use super::super::extractor::CipherFunctions;

        // Fetch a real player.js
        let player_url =
            "https://www.youtube.com/s/player/e06dea74/player_ias.vflset/en_US/base.js";

        let client = reqwest::Client::new();
        let player_js = client
            .get(player_url)
            .header(
                "User-Agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
            )
            .send()
            .await
            .expect("Failed to fetch player.js")
            .text()
            .await
            .expect("Failed to read response");

        println!("Fetched player.js: {} bytes", player_js.len());

        // Debug: look for iha in the player.js
        if let Some(idx) = player_js.find("var iha=") {
            println!("=== iha DEFINITION ===");
            println!("{}", &player_js[idx..idx + 500.min(player_js.len() - idx)]);
        } else if let Some(idx) = player_js.find("iha=function") {
            println!("=== iha DEFINITION (alt) ===");
            println!("{}", &player_js[idx..idx + 500.min(player_js.len() - idx)]);
        } else {
            println!("iha not found directly, searching for n-transform patterns...");
            // Look for the .get("n") pattern
            let re = regex::Regex::new(r#"\.get\("n"\)\)&&[^;]{0,100}"#).unwrap();
            for m in re.find_iter(&player_js).take(3) {
                println!("Found n-pattern: {}", m.as_str());
            }
        }

        // Try to extract cipher functions
        match CipherFunctions::extract(&player_js) {
            Ok(functions) => {
                println!(
                    "=== DECIPHER FUNCTION ({} bytes) ===",
                    functions.decipher_fn.len()
                );
                println!("{}", functions.decipher_fn);
                println!(
                    "\n=== N-TRANSFORM FUNCTION ({} bytes) ===",
                    functions.n_transform_fn.len()
                );
                println!(
                    "{}",
                    &functions.n_transform_fn[..2000.min(functions.n_transform_fn.len())]
                );
                if functions.n_transform_fn.len() > 2000 {
                    println!("... (truncated)");
                }
            }
            Err(e) => {
                panic!("Failed to extract functions: {}", e);
            }
        }
    }

    /// Test fetching a real YouTube player and extracting cipher functions.
    /// This test requires network access.
    #[tokio::test]
    #[ignore] // Run with: cargo test -- --ignored
    async fn test_fetch_real_player() {
        // This is a typical YouTube player URL pattern
        // You may need to update this URL if it becomes stale
        let player_url =
            "https://www.youtube.com/s/player/e06dea74/player_ias.vflset/en_US/base.js";

        let result = crate::cipher::PlayerContext::fetch(player_url).await;

        match result {
            Ok(mut ctx) => {
                println!("Successfully fetched player: {}", ctx.player_id());

                // Try a simple decipher - this won't produce valid results
                // without a real signature, but it should at least run
                let test_sig = "ABC123";
                match ctx.run_decipher(test_sig) {
                    Ok(deciphered) => {
                        println!("Decipher result: {}", deciphered);
                    }
                    Err(e) => {
                        println!("Decipher error (may be expected): {}", e);
                    }
                }

                // Try n-transform
                let test_n = "ABC123";
                match ctx.run_n_transform(test_n) {
                    Ok(transformed) => {
                        println!("N-transform result: {}", transformed);
                    }
                    Err(e) => {
                        println!("N-transform error (may be expected): {}", e);
                    }
                }
            }
            Err(e) => {
                panic!("Failed to fetch player: {}", e);
            }
        }
    }

    /// Test the cipher context with a mocked/simplified player.
    #[test]
    fn test_cipher_with_mock_functions() {
        use super::super::runtime::JsExecutor;

        let mut executor = JsExecutor::new().unwrap();

        // Mock decipher function that reverses the string
        let mock_code = r#"
            var helper = {
                reverse: function(a) { a.reverse(); },
                swap: function(a, b) { var c = a[0]; a[0] = a[b]; a[b] = c; }
            };
            function mockDecipher(a) {
                a = a.split("");
                helper.reverse(a);
                return a.join("");
            }
        "#;

        executor.execute_void(mock_code).unwrap();

        let result = executor.execute("mockDecipher('hello')").unwrap();
        assert_eq!(result, "olleh");
    }
}
