use crate::db::errors::DbError;

pub fn delta_encode(batch: &[i64]) -> Result<Vec<i64>, DbError>
{
    let min_val = *batch
        .iter()
        .min()
        .ok_or_else(|| DbError::Other(
            "delta_encode: cannot find minimum value in empty batch".to_string()
        ))?;

    let mut delta_encoded_vec: Vec<i64> = vec![min_val];
    let mut first_encountered = true;

    for elem in batch
    {
        // When we encounter our min we skip adding it since we already did this
        // before loop, but we skip ONLY ONCE, since we allow multiple same vals
        if first_encountered && min_val == *elem
        {
            first_encountered = false;
            continue;
        }
        // since we have minimum, we can make all differences non-negative
        delta_encoded_vec.push(*elem - min_val);
    }

    Ok(delta_encoded_vec)
}

pub fn vle_encode_u(buf: &mut Vec<u8>, mut val: u64)
{
    // We encode in little endian, so to decode we read until we find 0 at first
    // bit in read byte
    loop
    {
        buf.push((0x80 | (val & 0x7f)) as u8);

        // We might have lost information about 8th bit, thus we want to 
        // regain it so we move only 7 bits
        val = val >> 7;

        if val <= 0
        {
            break;
        }
    }

    if let Some(last) = buf.last_mut()
    {
        // We clear flag at last byte
        *last ^= 0x80;
    }
}

/// We assume that in buf: Vec<u8> we have enough data to be able to correctly 
/// construct u64 value
pub fn vle_decode_u(buf: &Vec<u8>) -> u64
{
    let mut out: u64 = 0;
    let mut shift: u64 = 0;

    for &byte in buf
    {
        out |=  ((byte & 0x7f) as u64) << shift;
        shift += 7;
    }

    out
}

pub fn vle_encode_i(buf: &mut Vec<u8>, val: i64)
{
    // We do ZigZag encoding, we will use this function only for the first value
    // in a batch, since all other values will be non-negative
    let new_val = val as u64;
    let new_val = if new_val & (1u64 << 63) != 0 {
        !(new_val << 1)
    }
    else {new_val << 1};

    vle_encode_u(buf, new_val);
}

pub fn vle_decode_i(buf: &Vec<u8>) -> i64
{
    let val = vle_decode_u(buf);

    if val & 1 == 0
    {
        (val >> 1) as i64
    }
    else 
    {
        !(val >> 1) as i64
    }
}