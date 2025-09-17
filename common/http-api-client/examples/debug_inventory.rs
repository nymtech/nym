use nym_http_api_client::registry;

fn main() {
    println!("Debugging HTTP Client Inventory");
    println!("================================");

    // Print all registered configurations
    registry::debug_print_inventory();

    // Also print the count
    println!(
        "\nTotal registered configs: {}",
        registry::registered_config_count()
    );

    // Show the detailed breakdown
    println!("\nDetailed configuration list:");
    for (i, (priority, ptr)) in registry::inspect_registered_configs().iter().enumerate() {
        println!(
            "  Config #{}: priority={}, function=0x{:x}",
            i + 1,
            priority,
            ptr
        );
    }
}
