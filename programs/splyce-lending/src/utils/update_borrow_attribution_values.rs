use anchor_lang::prelude::*;
use crate::state::{Obligation, Reserve};
use crate::error::ErrorCode;

/// This function updates the borrow attribution value on the ObligationCollateral and
/// the reserve.
///
/// Prerequisites:
/// - the collateral's market value must be refreshed
/// - the obligation's deposited_value must be refreshed
/// - the obligation's true_borrowed_value must be refreshed
///
/// Note that this function packs and unpacks deposit reserves.
// pub fn update_borrow_attribution_values<'info>(
//     obligation: &mut Obligation,
//     reserve_infos: &[AccountInfo<'info>],
// ) -> Result<(Option<Pubkey>, Option<Pubkey>)> {
//     let mut open_exceeded = None;
//     let mut close_exceeded = None;

//     for collateral in obligation.deposits.iter_mut() {
//         let reserve_info = reserve_infos
//             .iter()
//             .find(|info| info.key() == collateral.deposit_reserve)
//             .ok_or(ErrorCode::InvalidObligationCollateral)?;
//         let data = reserve_info.try_borrow_data()?;
//         msg!("Reserve data: {:?}", data);
        
//         // Skip the first 8 bytes (discriminator) when deserializing
//         let mut reserve = Reserve::try_from_slice(&data[8..])?;

//         reserve.attributed_borrow_value = reserve
//             .attributed_borrow_value
//             .checked_sub(collateral.attributed_borrow_value)
//             .ok_or(ErrorCode::MathOverflow)?;

//         if obligation.deposited_value > 0 {
//             collateral.attributed_borrow_value = collateral
//                 .market_value
//                 .checked_mul(obligation.unweighted_borrowed_value)
//                 .ok_or(ErrorCode::MathOverflow)?
//                 .checked_div(obligation.deposited_value)
//                 .ok_or(ErrorCode::MathOverflow)?;
//         } else {
//             collateral.attributed_borrow_value = 0;
//         }

//         reserve.attributed_borrow_value = reserve
//             .attributed_borrow_value
//             .checked_add(collateral.attributed_borrow_value)
//             .ok_or(ErrorCode::MathOverflow)?;

//         if reserve.attributed_borrow_value > reserve.config.attributed_borrow_limit_open.into() {
//             open_exceeded = Some(collateral.deposit_reserve);
//         }
//         if reserve.attributed_borrow_value > reserve.config.attributed_borrow_limit_close.into() {
//             close_exceeded = Some(collateral.deposit_reserve);
//         }

//         // When writing back, include the discriminator
//         // Serialize the updated reserve data
//         let mut updated_data = reserve.try_to_vec()?;
        
//         // Get the original data to retrieve the discriminator
//         let original_data = reserve_info.try_borrow_data()?;
//         let discriminator = &original_data[..8];
        
//         // Prepend the discriminator to the updated data
//         let mut final_data = Vec::with_capacity(8 + updated_data.len());
//         final_data.extend_from_slice(discriminator);
//         final_data.extend(updated_data);
        
//         // Write the final data back to the account
//         reserve_info.try_borrow_mut_data()?.copy_from_slice(&final_data);
//     }

//     Ok((open_exceeded, close_exceeded))
// }

pub fn update_borrow_attribution_values<'info>(
    obligation: &mut Obligation,
    reserve_infos: &[AccountInfo<'info>],
) -> Result<(Option<Pubkey>, Option<Pubkey>)> {
    let mut open_exceeded = None;
    let mut close_exceeded = None;

    for collateral in obligation.deposits.iter_mut() {
        let reserve_info = reserve_infos
            .iter()
            .find(|info| info.key() == collateral.deposit_reserve)
            .ok_or(ErrorCode::InvalidObligationCollateral)?;
        
        let data = reserve_info.try_borrow_data()?;
        msg!("Reserve data length: {}", data.len());
        
        if data.len() < 8 {
            msg!("Data length is less than 8 bytes");
            return Err(ErrorCode::InvalidReserveData.into());
        }
        
        // Skip the 8-byte discriminator
        let mut reserve: Reserve = Reserve::try_from_slice(&data[8..])?;
        
        msg!("Successfully deserialized Reserve");
        
        // Proceed with the rest of the logic
        reserve.attributed_borrow_value = reserve
            .attributed_borrow_value
            .checked_sub(collateral.attributed_borrow_value)
            .ok_or(ErrorCode::MathOverflow)?;

        // Serialize the updated Reserve
        let mut updated_data = reserve.try_to_vec()?;
        let final_data_len = 8 + updated_data.len();
        
        msg!("Updated data length: {}", updated_data.len());
        msg!("Final data length (with discriminator): {}", final_data_len);
        msg!("Original account data length: {}", data.len());
        
        if final_data_len > data.len() {
            msg!("Updated data is larger than original data");
            return Err(ErrorCode::InvalidReserveData.into());
        }
        
        // Write back the updated data
        let mut final_data = data.to_vec();
        final_data[8..final_data_len].copy_from_slice(&updated_data);
        reserve_info.try_borrow_mut_data()?.copy_from_slice(&final_data);
        
        msg!("Successfully updated reserve data");
    }

    Ok((open_exceeded, close_exceeded))
}


// pub fn update_borrow_attribution_values<'info>(
//     obligation: &mut Obligation,
//     reserve_infos: &[AccountInfo<'info>],
// ) -> Result<(Option<Pubkey>, Option<Pubkey>)> {
//     let mut open_exceeded = None;
//     let mut close_exceeded = None;

//     for collateral in obligation.deposits.iter_mut() {
//         let reserve_info = reserve_infos
//             .iter()
//             .find(|info| info.key() == collateral.deposit_reserve)
//             .ok_or(ErrorCode::InvalidObligationCollateral)?;
        
//         let data = reserve_info.try_borrow_data()?;
//         msg!("Reserve data length: {}", data.len());
        
//         if data.len() < 8 {
//             msg!("Data length is less than 8 bytes");
//             return Err(ErrorCode::InvalidReserveData.into());
//         }
        
//         msg!("Discriminator: {:?}", &data[..8]);
        
//         // Try to deserialize the Reserve struct
//         let reserve_result = Reserve::try_from_slice(&data[8..]);
        
//         match reserve_result {
//             Ok(mut reserve) => {
//                 msg!("Successfully deserialized Reserve");
//                 msg!("Reserve struct size: {}", std::mem::size_of::<Reserve>());
                
//                 // Proceed with the rest of the logic
//                 reserve.attributed_borrow_value = reserve
//                     .attributed_borrow_value
//                     .checked_sub(collateral.attributed_borrow_value)
//                     .ok_or(ErrorCode::MathOverflow)?;

//                 // Serialize the updated Reserve
//                 let mut updated_data = reserve.try_to_vec()?;
//                 let final_data_len = 8 + updated_data.len();
                
//                 msg!("Updated data length: {}", updated_data.len());
//                 msg!("Final data length (with discriminator): {}", final_data_len);
//                 msg!("Original account data length: {}", data.len());
                
//                 if final_data_len > data.len() {
//                     msg!("Updated data is larger than original data");
//                     return Err(ErrorCode::InvalidReserveData.into());
//                 }
                
//                 // Write back the updated data
//                 let mut final_data = data.to_vec();
//                 final_data[8..final_data_len].copy_from_slice(&updated_data);
//                 reserve_info.try_borrow_mut_data()?.copy_from_slice(&final_data);
                
//                 msg!("Successfully updated reserve data");
//             },
//             Err(e) => {
//                 msg!("Error deserializing Reserve: {:?}", e);
//                 msg!("Reserve struct size: {}", std::mem::size_of::<Reserve>());
//                 msg!("First 100 bytes of data after discriminator: {:?}", &data[8..].iter().take(100).collect::<Vec<_>>());
//                 return Err(ErrorCode::InvalidReserveData.into());
//             }
//         };
//     }

//     Ok((open_exceeded, close_exceeded))
// }

// pub fn update_borrow_attribution_values<'info>(
//     obligation: &mut Obligation,
//     reserve_infos: &[AccountInfo<'info>],
// ) -> Result<(Option<Pubkey>, Option<Pubkey>)> {
//     let mut open_exceeded = None;
//     let mut close_exceeded = None;

//     for collateral in obligation.deposits.iter_mut() {
//         let reserve_info = reserve_infos
//             .iter()
//             .find(|info| info.key() == collateral.deposit_reserve)
//             .ok_or(ErrorCode::InvalidObligationCollateral)?;

//         // Borrow the raw data from the reserve_info account
//         let data = reserve_info.try_borrow_data()?;
//         msg!("Reserve raw data (first 20 bytes): {:?}", &data[..20]);

//         // Initialize offset to skip the discriminator (first 8 bytes)
//         let mut offset = 8;

//         // Helper function to move through the raw data safely
//         fn advance_offset<'a>(data: &'a [u8], offset: &mut usize, len: usize) -> Result<&'a [u8]> {
//             if *offset + len > data.len() {
//                 msg!("Attempting to read beyond data length");
//                 return Err(ErrorCode::InvalidReserveData.into());
//             }
//             let slice = &data[*offset..*offset + len];
//             *offset += len;
//             Ok(slice)
//         }

//         // Deserialize version (u8)
//         let version = data[offset];
//         offset += 1;
//         msg!("Version: {}", version);

//         // Deserialize last_update (assuming 9 bytes: 8 for slot, 1 for stale)
//         let last_update_slot = u64::from_le_bytes(
//             advance_offset(&data, &mut offset, 8)?
//                 .try_into()
//                 .map_err(|_| ErrorCode::InvalidReserveData)?,
//         );
//         let last_update_stale_byte = data[offset];
//         offset += 1;
//         msg!("Last Update Slot: {}, Stale Byte: {}", last_update_slot, last_update_stale_byte);

//         // Validate stale byte
//         if last_update_stale_byte != 0 && last_update_stale_byte != 1 {
//             msg!("Invalid bool representation for stale: {}", last_update_stale_byte);
//             return Err(ErrorCode::InvalidBoolRepresentation.into());
//         }
//         let last_update_stale = last_update_stale_byte != 0;
//         msg!("Last Update Stale: {}", last_update_stale);

//         // Deserialize lending_market (Pubkey - 32 bytes)
//         let lending_market_pubkey = Pubkey::new_from_array(
//             advance_offset(&data, &mut offset, 32)?
//                 .try_into()
//                 .map_err(|_| ErrorCode::InvalidReserveData)?,
//         );
//         msg!("Lending Market Pubkey: {:?}", lending_market_pubkey);

//         // Deserialize liquidity (ReserveLiquidity struct - assumed 190 bytes)
//         let liquidity_data = advance_offset(&data, &mut offset, 190)?;
//         msg!("Liquidity Data (first 20 bytes): {:?}", &liquidity_data[..20]);

//         // Deserialize collateral (ReserveCollateral struct - 72 bytes)
//         let collateral_data = advance_offset(&data, &mut offset, 72)?;
//         msg!("Collateral Data (first 20 bytes): {:?}", &collateral_data[..20]);

//         // Deserialize config (ReserveConfig struct - 165 bytes)
//         let config_data = advance_offset(&data, &mut offset, 165)?;
//         msg!("Config Data (first 20 bytes): {:?}", &config_data[..20]);

//         // Deserialize attributed_borrow_value (u128 - 16 bytes)
//         let attributed_borrow_value = u128::from_le_bytes(
//             advance_offset(&data, &mut offset, 16)?
//                 .try_into()
//                 .map_err(|_| ErrorCode::InvalidReserveData)?,
//         );
//         msg!("Attributed Borrow Value: {}", attributed_borrow_value);

//         // Deserialize key (u64 - 8 bytes)
//         let key = u64::from_le_bytes(
//             advance_offset(&data, &mut offset, 8)?
//                 .try_into()
//                 .map_err(|_| ErrorCode::InvalidReserveData)?,
//         );
//         msg!("Key: {}", key);

//         // Deserialize mock_pyth_feed (Pubkey - 32 bytes)
//         let mock_pyth_feed = Pubkey::new_from_array(
//             advance_offset(&data, &mut offset, 32)?
//                 .try_into()
//                 .map_err(|_| ErrorCode::InvalidReserveData)?,
//         );
//         msg!("Mock Pyth Feed Pubkey: {:?}", mock_pyth_feed);

//         // If any data remains, print it (to check for padding or extra data)
//         if offset < data.len() {
//             let remaining_data = &data[offset..];
//             msg!("Remaining Data ({} bytes): {:?}", remaining_data.len(), remaining_data);
//         }

//         // At this point, you can manually inspect the `stale` field
//         // If stale is invalid, you need to investigate why it's being set to 32

//         // Uncomment the following line to attempt deserialization after manual inspection
//         // let reserve = Reserve::try_from_slice(&data[8..])?;

//         // Rest of your logic goes here
//         // ...
//     }

//     Ok((open_exceeded, close_exceeded))
// }