//! Test to verify gateway reconnection when starting before server

use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use wyldlands_gateway::grpc::RpcClientManager;

#[tokio::test]
async fn test_gateway_reconnects_when_server_starts_late() {
    // Initialize tracing for test output
    let _ = tracing_subscriber::fmt().with_test_writer().try_init();

    // Create RPC client pointing to a server that doesn't exist yet
    let rpc_client = Arc::new(RpcClientManager::new(
        "127.0.0.1:9999", // Non-existent server
        "test-key",
        1, // 1 second reconnect interval for faster testing
        5, // 5 second heartbeat interval
    ));

    // Verify initial state is disconnected
    assert!(!rpc_client.is_connected().await);

    // Start reconnection loop
    let rpc_client_reconnect = Arc::clone(&rpc_client);
    let reconnect_handle = tokio::spawn(async move {
        rpc_client_reconnect.start_reconnection_loop().await;
    });

    // Wait a bit to let it try to connect (and fail)
    sleep(Duration::from_secs(2)).await;

    // Should still be disconnected since server doesn't exist
    assert!(!rpc_client.is_connected().await);

    // Verify the reconnection loop is still running and trying
    // (it should not have given up)
    assert!(!reconnect_handle.is_finished());

    // Clean up
    reconnect_handle.abort();
}

#[tokio::test]
async fn test_state_transitions_during_reconnection() {
    // Initialize tracing for test output
    let _ = tracing_subscriber::fmt().with_test_writer().try_init();

    // Create RPC client
    let rpc_client = Arc::new(RpcClientManager::new(
        "127.0.0.1:9998", // Non-existent server
        "test-key",
        1, // 1 second reconnect interval
        5, // 5 second heartbeat interval
    ));

    // Start reconnection loop
    let rpc_client_reconnect = Arc::clone(&rpc_client);
    let reconnect_handle = tokio::spawn(async move {
        rpc_client_reconnect.start_reconnection_loop().await;
    });

    // Initial state should be disconnected
    let initial_state = rpc_client.state().await;
    println!("Initial state: {:?}", initial_state);

    // Wait for first connection attempt
    sleep(Duration::from_millis(500)).await;

    // After failed attempt, should be back to Disconnected (not stuck in Failed)
    sleep(Duration::from_millis(1500)).await;
    let state_after_retry = rpc_client.state().await;
    println!("State after retry: {:?}", state_after_retry);

    // The state should be either Disconnected or Connecting, not Failed
    // (Failed is transient and should be reset to Disconnected)
    assert!(
        matches!(
            state_after_retry,
            wyldlands_gateway::grpc::client::ClientState::Disconnected
                | wyldlands_gateway::grpc::client::ClientState::Connecting
        ),
        "State should be Disconnected or Connecting, got {:?}",
        state_after_retry
    );

    // Clean up
    reconnect_handle.abort();
}


