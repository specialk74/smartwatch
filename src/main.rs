// See the "macOS permissions note" in README.md before running this on macOS
// Big Sur or later.

use btleplug::api::{
    Central, CharPropFlags, Characteristic, Manager as _, Peripheral, ScanFilter, WriteType,
};
use btleplug::platform::Manager;
use chrono::{Datelike, Timelike, Utc};
use futures::stream::StreamExt;
use std::error::Error;
use std::time::Duration;
use tokio::time;
use uuid::Uuid;

/// Only devices whose name contains this string will be tried.
const PERIPHERAL_NAME_MATCH_FILTER: &str = "Amazfit GTS 4 Mini";
/// UUID of the characteristic for which we should subscribe to notifications.
const NOTIFY_CHARACTERISTIC_UUID: Uuid = Uuid::from_u128(0x6e400002_b534_f393_67a9_e50e24dccA9e);
const TIME_CHARACTERISTIC_UUID: Uuid = Uuid::from_u128(0x00002a2b_0000_1000_8000_00805f9b34fb); // 00002a2b-0000-1000-8000-00805f9b34fb

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    pretty_env_logger::init();

    let manager = Manager::new().await?;
    let adapter_list = manager.adapters().await?;
    if adapter_list.is_empty() {
        eprintln!("No Bluetooth adapters found");
    }

    for adapter in adapter_list.iter() {
        println!("Starting scan...");
        adapter
            .start_scan(ScanFilter::default())
            .await
            .expect("Can't scan BLE adapter for connected devices...");
        time::sleep(Duration::from_secs(2)).await;
        let peripherals = adapter.peripherals().await?;

        if peripherals.is_empty() {
            eprintln!("->>> BLE peripheral devices were not found, sorry. Exiting...");
        } else {
            // All peripheral devices in range.
            for peripheral in peripherals.iter() {
                let properties = peripheral.properties().await?;
                let is_connected = peripheral.is_connected().await?;
                let local_name = properties
                    .unwrap()
                    .local_name
                    .unwrap_or(String::from("(peripheral name unknown)"));
                /* println!(
                    "Peripheral {:?} is connected: {:?}",
                    &local_name, is_connected
                ); */
                // Check if it's the peripheral we want.
                if local_name.contains(PERIPHERAL_NAME_MATCH_FILTER) {
                    println!("Found matching peripheral {:?}...", &local_name);
                    if !is_connected {
                        // Connect if we aren't already connected.
                        if let Err(err) = peripheral.connect().await {
                            eprintln!("Error connecting to peripheral, skipping: {}", err);
                            continue;
                        }
                    }
                    let is_connected = peripheral.is_connected().await?;
                    println!(
                        "Now connected ({:?}) to peripheral {:?}.",
                        is_connected, &local_name
                    );
                    if is_connected {
                        println!("Discover peripheral {:?} services...", local_name);
                        peripheral.discover_services().await?;
                        for characteristic in peripheral.characteristics() {
                            //println!("Checking characteristic {:?}", characteristic);
                            // Subscribe to notifications from the characteristic with the selected
                            // UUID.
                            if characteristic.properties.contains(CharPropFlags::READ)
                                && characteristic.uuid == TIME_CHARACTERISTIC_UUID
                            {
                                println!("Reading characteristic {:?}", characteristic.uuid);
                                let value = peripheral.read(&characteristic).await?;
                                println!(
                                    "Read value from {:?} [{:?}]: {:?}",
                                    local_name, characteristic.uuid, value
                                );
                            }
                            if characteristic.uuid == TIME_CHARACTERISTIC_UUID {
                                set_current_time(peripheral, &characteristic).await?;

                                /* println!("Write characteristic {:?}", characteristic.uuid);
                                let value = 0x07e8; // 2022
                                let value = peripheral
                                    .write(
                                        &characteristic,
                                        &[
                                            0, // Anno
                                            0, // Anno
                                            0, // Mese
                                            0, // Giorno del mese
                                            0, // Ora
                                            0, // Minuti
                                            0, // Secondi
                                            0, //
                                            0, //
                                            0, //
                                            1, //
                                        ],
                                        WriteType::WithResponse,
                                    )
                                    .await;
                                println!("Value written: {:?}", value); */
                            }
                            if characteristic.properties.contains(CharPropFlags::READ)
                                && characteristic.uuid == TIME_CHARACTERISTIC_UUID
                            {
                                println!("Reading characteristic {:?}", characteristic.uuid);
                                let value = peripheral.read(&characteristic).await?;
                                println!(
                                    "Read value from {:?} [{:?}]: {:?}",
                                    local_name, characteristic.uuid, value
                                );
                            }
                            /* if characteristic.uuid == NOTIFY_CHARACTERISTIC_UUID
                                && characteristic.properties.contains(CharPropFlags::NOTIFY)
                            {
                                println!("Subscribing to characteristic {:?}", characteristic.uuid);
                                peripheral.subscribe(&characteristic).await?;
                                // Print the first 4 notifications received.
                                let mut notification_stream =
                                    peripheral.notifications().await?.take(4);
                                // Process while the BLE connection is not broken or stopped.
                                while let Some(data) = notification_stream.next().await {
                                    println!(
                                        "Received data from {:?} [{:?}]: {:?}",
                                        local_name, data.uuid, data.value
                                    );
                                }
                            } */
                        }
                        println!("Disconnecting from peripheral {:?}...", local_name);
                        peripheral.disconnect().await?;
                    }
                }
            }
        }
    }
    Ok(())
}

async fn set_current_time(
    peripheral: &impl Peripheral,
    characteristic: &Characteristic,
) -> Result<(), Box<dyn std::error::Error>> {
    // Get the current time
    let now = Utc::now();
    let year = now.year() as u16;
    let month = now.month() as u8;
    let day = now.day() as u8;
    let hour = now.hour() as u8;
    let minute = now.minute() as u8;
    let second = now.second() as u8;
    let day_of_week = now.weekday().num_days_from_sunday() as u8; // Sunday = 1, Monday = 2, ..., Saturday = 7
    let fractions = 0u8; // Fractions of a second
    let adjust_reason = 1u8; // No adjustment

    // Construct the data to write
    let data = [
        (year & 0xFF) as u8,        // Year (LSB)
        ((year >> 8) & 0xFF) as u8, // Year (MSB)
        month,                      // Month
        day,                        // Day
        hour,                       // Hours
        minute,                     // Minutes
        second,                     // Seconds
        day_of_week,                // Day of Week
        fractions,                  // Fractions of a second
        0,
        adjust_reason, // Adjust Reason
    ];

    data.iter().for_each(|b| print!("{:02X} ", b));

    // Write the data to the characteristic
    match peripheral
        .write(characteristic, &data, WriteType::WithResponse)
        .await
    {
        Ok(_) => println!("Current time written successfully"),
        Err(e) => eprintln!("Failed to write current time: {:?}", e),
    }

    Ok(())
}
