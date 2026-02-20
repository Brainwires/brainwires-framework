//! Multi-API Orchestration Example
//!
//! This example demonstrates orchestrating multiple API calls with
//! conditional logic - a common pattern where the result of one API
//! call determines which subsequent calls to make.
//!
//! ## Scenario
//!
//! We have three "APIs":
//! - User service: Get user preferences
//! - Weather service: Get weather for a location
//! - Activity service: Suggest activities based on weather
//!
//! The script fetches user preferences, checks the weather for their
//! location, and suggests appropriate activities.
//!
//! Run with: `cargo run --example multi_api`

use tool_orchestrator::{ExecutionLimits, ToolOrchestrator};

fn main() {
    println!("=== Multi-API Orchestration Example ===\n");

    let mut orchestrator = ToolOrchestrator::new();

    // Simulated User Service API
    orchestrator.register_executor("get_user_preferences", |input| {
        let user_id = input.as_str().unwrap_or("unknown");

        let prefs = match user_id {
            "alice" => r#"{"location":"Seattle","interests":["hiking","coffee","tech"],"indoor_preference":false}"#,
            "bob" => r#"{"location":"Miami","interests":["beach","surfing","nightlife"],"indoor_preference":false}"#,
            "carol" => r#"{"location":"Denver","interests":["skiing","craft beer","concerts"],"indoor_preference":true}"#,
            _ => r#"{"location":"Unknown","interests":[],"indoor_preference":true}"#,
        };

        Ok(prefs.to_string())
    });

    // Simulated Weather Service API
    orchestrator.register_executor("get_weather", |input| {
        let location = input.as_str().unwrap_or("Unknown");

        let weather = match location {
            "Seattle" => r#"{"temp":55,"condition":"rainy","humidity":85}"#,
            "Miami" => r#"{"temp":82,"condition":"sunny","humidity":70}"#,
            "Denver" => r#"{"temp":45,"condition":"snowy","humidity":40}"#,
            _ => r#"{"temp":70,"condition":"unknown","humidity":50}"#,
        };

        Ok(weather.to_string())
    });

    // Simulated Activity Suggestion API
    orchestrator.register_executor("suggest_activities", |input| {
        // Input is expected to be a JSON-like string with condition and interests
        let input_str = input.as_str().unwrap_or("{}");

        // Simple parsing for demo
        let is_outdoor_weather = input_str.contains("sunny") || input_str.contains("clear");
        let is_rainy = input_str.contains("rainy");
        let is_snowy = input_str.contains("snowy");

        let activities = if is_snowy {
            vec!["Skiing", "Snowboarding", "Hot cocoa at a cafe", "Indoor climbing"]
        } else if is_rainy {
            vec!["Visit a museum", "Coffee shop hopping", "Indoor rock climbing", "Movie marathon"]
        } else if is_outdoor_weather {
            vec!["Beach day", "Hiking", "Outdoor dining", "Park picnic"]
        } else {
            vec!["Local exploration", "Try a new restaurant", "Visit a bookstore"]
        };

        Ok(format!("[{}]", activities.iter().map(|a| format!("\"{}\"", a)).collect::<Vec<_>>().join(",")))
    });

    // Simulated notification service
    orchestrator.register_executor("send_notification", |input| {
        let message = input.as_str().unwrap_or("No message");
        println!("  [NOTIFICATION] {}", message);
        Ok("sent".to_string())
    });

    // The orchestration script - this is what an LLM would generate
    let script = r#"
        // Helper function to join array elements with a separator
        fn join_array(arr, sep) {
            let result = "";
            for i in 0..arr.len() {
                if i > 0 {
                    result += sep;
                }
                result += arr[i];
            }
            result
        }

        // Process activity suggestions for multiple users
        let users = ["alice", "bob", "carol"];
        let results = [];

        for user in users {
            // Step 1: Get user preferences
            let prefs_json = get_user_preferences(user);

            // Extract location (simple parsing)
            let location = "";
            if prefs_json.contains("Seattle") {
                location = "Seattle";
            } else if prefs_json.contains("Miami") {
                location = "Miami";
            } else if prefs_json.contains("Denver") {
                location = "Denver";
            }

            // Step 2: Get weather for their location
            let weather_json = get_weather(location);

            // Extract condition
            let condition = "unknown";
            if weather_json.contains("rainy") {
                condition = "rainy";
            } else if weather_json.contains("sunny") {
                condition = "sunny";
            } else if weather_json.contains("snowy") {
                condition = "snowy";
            }

            // Extract temperature using find
            let temp = 70;
            let temp_idx = weather_json.index_of("temp\":");
            if temp_idx >= 0 {
                let temp_part = weather_json.sub_string(temp_idx + 6, 2);
                let parsed = temp_part.parse_int();
                if parsed != () {
                    temp = parsed;
                }
            }

            // Step 3: Get activity suggestions based on weather
            let activities_json = suggest_activities(condition);

            // Step 4: Conditional notification
            if temp < 50 {
                send_notification(`${user}: Bundle up! It's ${temp}°F in ${location}`);
            } else if temp > 80 {
                send_notification(`${user}: Stay cool! It's ${temp}°F in ${location}`);
            }

            // Build result for this user
            results.push(`${user} (${location}):
  Weather: ${condition}, ${temp}°F
  Suggested: ${activities_json}`);
        }

        // Build results list
        let results_list = join_array(results, "\n\n");

        // Return consolidated results
        `=== Personalized Activity Recommendations ===

${results_list}

---
Processed ${users.len()} users with conditional notifications.`
    "#;

    println!("Running multi-API orchestration...\n");

    let result = orchestrator
        .execute(script, ExecutionLimits::default())
        .expect("Script execution failed");

    println!("\n{}", result.output);

    println!("\n=== Execution Summary ===");
    println!("Total tool calls: {}", result.tool_calls.len());
    println!("Execution time: {}ms", result.execution_time_ms);

    println!("\n=== Call Breakdown ===");
    let mut call_counts: std::collections::HashMap<&str, u32> = std::collections::HashMap::new();
    for call in &result.tool_calls {
        *call_counts.entry(&call.tool_name).or_insert(0) += 1;
    }
    for (tool, count) in call_counts {
        println!("  {}: {} calls", tool, count);
    }

    println!("\n=== Why This Matters ===");
    println!("Traditional approach: 12+ round trips to the LLM");
    println!("PTC approach: 1 round trip with orchestrated logic");
    println!("The conditional notifications happened automatically based on weather!");
}
