pub fn read_slice<'a>(bytes: &'a [u8], pos: &mut usize, length: usize) -> Result<&'a [u8], String> {
    let end = pos.checked_add(length).ok_or("length overflow")?;
    let slice = bytes
        .get(*pos..end)
        .ok_or("unexpected eof: slice out of range")?;
    *pos = end;
    Ok(slice)
}

pub fn read_u32(bytes: &[u8], pos: &mut usize) -> Result<u32, String> {
    let slice = read_slice(bytes, pos, 4)?;
    Ok(u32::from_le_bytes(slice.try_into().unwrap()))
}

pub fn read_i64(bytes: &[u8], pos: &mut usize) -> Result<i64, String> {
    let slice = read_slice(bytes, pos, 8)?;
    Ok(i64::from_le_bytes(slice.try_into().unwrap()))
}

pub fn read_f64(bytes: &[u8], pos: &mut usize) -> Result<f64, String> {
    let slice = read_slice(bytes, pos, 8)?;
    Ok(f64::from_le_bytes(slice.try_into().unwrap()))
}
