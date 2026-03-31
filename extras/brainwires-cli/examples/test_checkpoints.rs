use anyhow::Result;
use brainwires_cli::types::message::{Message, MessageContent, Role};
use brainwires_cli::utils::checkpoint::CheckpointManager;
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Testing Checkpoint System ===\n");

    // Initialize checkpoint manager
    let manager = CheckpointManager::new()?;
    println!("✓ CheckpointManager initialized\n");

    // Clean up any existing test checkpoints
    let existing = manager.list_checkpoints("test-conversation-123").await?;
    let existing_count = existing.len();
    for cp in existing {
        let _ = manager.delete_checkpoint(&cp.id).await;
    }
    println!(
        "✓ Cleaned up {} existing test checkpoint(s)\n",
        existing_count
    );

    // Create sample messages
    let messages = vec![
        Message {
            role: Role::User,
            content: MessageContent::Text("Hello, how are you?".to_string()),
            name: None,
            metadata: None,
        },
        Message {
            role: Role::Assistant,
            content: MessageContent::Text("I'm doing great! How can I help you?".to_string()),
            name: None,
            metadata: None,
        },
    ];

    // Create metadata
    let mut metadata = HashMap::new();
    metadata.insert("model".to_string(), "llama-3.3-70b".to_string());
    metadata.insert("user".to_string(), "test-user".to_string());

    // Test 1: Create a checkpoint
    println!("Test 1: Creating checkpoint...");
    let checkpoint_id = manager
        .create_checkpoint(
            Some("Test Checkpoint".to_string()),
            "test-conversation-123".to_string(),
            messages.clone(),
            metadata.clone(),
        )
        .await?;
    println!("✓ Checkpoint created: {}\n", checkpoint_id);

    // Test 2: Load the checkpoint
    println!("Test 2: Loading checkpoint...");
    let loaded = manager.load_checkpoint(&checkpoint_id).await?;
    println!("✓ Checkpoint loaded:");
    println!("  - ID: {}", loaded.id);
    println!("  - Name: {:?}", loaded.name);
    println!("  - Conversation ID: {}", loaded.conversation_id);
    println!("  - Messages: {}", loaded.messages.len());
    println!("  - Metadata: {:?}\n", loaded.metadata);

    // Test 3: List checkpoints
    println!("Test 3: Listing checkpoints...");
    let checkpoints = manager.list_checkpoints("test-conversation-123").await?;
    println!("✓ Found {} checkpoint(s):", checkpoints.len());
    for (i, cp) in checkpoints.iter().enumerate() {
        println!(
            "  {}. {} - {} messages (created: {})",
            i + 1,
            cp.name.as_ref().unwrap_or(&cp.id[..8].to_string()),
            cp.messages.len(),
            chrono::DateTime::from_timestamp(cp.created_at, 0)
                .map(|dt| dt.to_rfc3339())
                .unwrap_or_else(|| "unknown".to_string())
        );
    }
    println!();

    // Test 4: Create another checkpoint
    println!("Test 4: Creating second checkpoint...");
    // Add a small delay to ensure different timestamps
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    let messages2 = vec![
        messages[0].clone(),
        messages[1].clone(),
        Message {
            role: Role::User,
            content: MessageContent::Text("What's the weather?".to_string()),
            name: None,
            metadata: None,
        },
    ];
    let checkpoint_id2 = manager
        .create_checkpoint(
            Some("Second Checkpoint".to_string()),
            "test-conversation-123".to_string(),
            messages2,
            metadata.clone(),
        )
        .await?;
    println!("✓ Second checkpoint created: {}\n", checkpoint_id2);

    // Test 5: List all checkpoints again
    println!("Test 5: Listing all checkpoints...");
    let all_checkpoints = manager.list_checkpoints("test-conversation-123").await?;
    println!("✓ Found {} checkpoint(s):", all_checkpoints.len());
    for (i, cp) in all_checkpoints.iter().enumerate() {
        println!(
            "  {}. {} - {} messages",
            i + 1,
            cp.name.as_ref().unwrap_or(&cp.id[..8].to_string()),
            cp.messages.len()
        );
    }
    println!();

    // Test 6: Restore from first checkpoint
    println!("Test 6: Restoring from first checkpoint...");
    let restored = manager.restore_checkpoint(&checkpoint_id).await?;
    println!("✓ Restored checkpoint:");
    println!("  - Name: {:?}", restored.name);
    println!("  - Messages: {}", restored.messages.len());
    assert_eq!(restored.messages.len(), 2);
    println!();

    // Test 7: Get latest checkpoint
    println!("Test 7: Getting latest checkpoint...");
    let latest = manager
        .get_latest_checkpoint("test-conversation-123")
        .await?;
    if let Some(cp) = latest {
        println!("✓ Latest checkpoint:");
        println!("  - Name: {:?}", cp.name);
        println!("  - Messages: {}", cp.messages.len());
        assert_eq!(cp.messages.len(), 3); // Second checkpoint has 3 messages
    }
    println!();

    // Test 8: Clean up - delete checkpoints
    println!("Test 8: Cleaning up...");
    manager.delete_checkpoint(&checkpoint_id).await?;
    manager.delete_checkpoint(&checkpoint_id2).await?;
    println!("✓ Checkpoints deleted\n");

    // Verify deletion
    let final_checkpoints = manager.list_checkpoints("test-conversation-123").await?;
    println!("✓ Final checkpoint count: {}", final_checkpoints.len());
    assert_eq!(final_checkpoints.len(), 0);

    println!("\n=== All tests passed! ===");

    Ok(())
}
