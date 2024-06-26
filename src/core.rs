pub use bincode;
pub use indexmap;
pub use log;
pub use serde;
pub use serde_derive;
pub use serde_json;

////////////////////////////////////////////////////////////////////////////////////////////////////
// Filesystem

pub mod platform;

////////////////////////////////////////////////////////////////////////////////////////////////////
// Debugging and performance

/// This is pretty similar to the dbg! macro only that dformat! returns a string
#[macro_export]
macro_rules! dformat {
    ($x:expr) => {
        format!("{} = {:?}", stringify!($x), $x)
    };
}

#[macro_export]
macro_rules! dformat_pretty {
    ($x:expr) => {
        format!("{} = {:#?}", stringify!($x), $x)
    };
}

pub struct TimerScoped {
    log_message: String,
    creation_time: f64,
}

impl Drop for TimerScoped {
    fn drop(&mut self) {
        let duration_since_creation = platform::timer_current_time_seconds() - self.creation_time;
        log::debug!(
            "{}: {:.3}ms",
            self.log_message,
            duration_since_creation * 1000.0
        );
    }
}

impl TimerScoped {
    pub fn new_scoped(output_text: &str, _use_logger: bool) -> TimerScoped {
        TimerScoped {
            log_message: output_text.to_owned(),
            creation_time: platform::timer_current_time_seconds(),
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Convenience Serialization / Deserialization

pub fn serialize_to_binary<T>(data: &T) -> Vec<u8>
where
    T: serde::Serialize,
{
    bincode::serialize(data).unwrap()
}

pub fn serialize_to_json<T>(data: &T) -> String
where
    T: serde::Serialize,
{
    serde_json::to_string_pretty(data).unwrap()
}

pub fn deserialize_from_binary<T>(data: &[u8]) -> T
where
    for<'de> T: serde::Deserialize<'de>,
{
    bincode::deserialize(data).unwrap()
}

pub fn deserialize_from_json<T>(json: &str) -> T
where
    for<'de> T: serde::Deserialize<'de>,
{
    serde_json::from_str(json).unwrap()
}

#[cfg(not(target_arch = "wasm32"))]
pub fn serialize_to_binary_file<T>(data: &T, filepath: &str)
where
    T: serde::Serialize,
{
    let encoded_data = bincode::serialize(data).unwrap_or_else(|error| {
        panic!(
            "Could not encode data for serializing to binary file '{}': {}",
            filepath, error
        )
    });
    std::fs::write(filepath, encoded_data).unwrap_or_else(|error| {
        panic!(
            "Could not write serialized data to binary file '{}': {}",
            filepath, error
        )
    });
}

#[cfg(not(target_arch = "wasm32"))]
pub fn serialize_to_json_file<T>(data: &T, filepath: &str)
where
    T: serde::Serialize,
{
    let output_string = serde_json::to_string_pretty(data).unwrap_or_else(|error| {
        panic!(
            "Could not deserialize data to json for writing to '{}': {}",
            filepath, error
        )
    });
    std::fs::write(filepath, output_string).unwrap_or_else(|error| {
        panic!(
            "Could write data string to json file '{}': {}",
            filepath, error
        )
    });
}

#[cfg(not(target_arch = "wasm32"))]
pub fn deserialize_from_binary_file<T>(filepath: &str) -> T
where
    for<'de> T: serde::Deserialize<'de>,
{
    let file_content = platform::read_file_whole(filepath).unwrap_or_else(|error| {
        panic!(
            "Could not open binary file '{}' for deserialization: {}",
            filepath, error
        )
    });
    bincode::deserialize(&file_content).unwrap_or_else(|error| {
        panic!(
            "Could not deserialize from binary file '{}': {}",
            filepath, error
        )
    })
}

#[cfg(not(target_arch = "wasm32"))]
pub fn deserialize_from_json_file<T>(filepath: &str) -> T
where
    for<'de> T: serde::Deserialize<'de>,
{
    let file_content = platform::read_file_whole(filepath).unwrap_or_else(|error| {
        panic!(
            "Could not open json file '{}' for deserialization: {}",
            filepath, error
        )
    });
    serde_json::from_reader(std::io::Cursor::new(file_content)).unwrap_or_else(|error| {
        panic!(
            "Could not deserialize from json file '{}': {}",
            filepath, error
        )
    })
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// DESERIALIZER

#[derive(Clone)]
pub struct Deserializer<'a> {
    data: &'a [u8],
}

impl Deserializer<'_> {
    pub fn new(data: &[u8]) -> Deserializer {
        Deserializer { data }
    }

    pub fn get_remaining_data(&self) -> &[u8] {
        self.data
    }

    pub fn skip_bytes(&mut self, byte_count: usize) -> Result<(), String> {
        if byte_count > self.data.len() {
            return Err(format!(
                "Cannot not skip {} bytes, internal buffer has only {} bytes left",
                byte_count,
                self.data.len()
            ));
        }
        self.data = &self.data[byte_count..];
        Ok(())
    }

    pub fn skip<T>(&mut self) -> Result<(), String>
    where
        for<'de> T: serde::Deserialize<'de>,
    {
        let size = std::mem::size_of::<T>();
        self.skip_bytes(size)
    }

    pub fn deserialize<T>(&mut self) -> Result<T, String>
    where
        for<'de> T: serde::Deserialize<'de>,
    {
        let size = std::mem::size_of::<T>();
        if size > self.data.len() {
            return Err(format!(
                "Cannot not deserialize {} which required {} bytes, but internal buffer has only {} bytes left",
                std::any::type_name::<T>(),
                size,
                self.data.len()
            ));
        }
        let result = bincode::deserialize::<T>(self.data).map_err(|error| {
            format!(
                "Could not deserialize {}: {}",
                std::any::type_name::<T>(),
                error
            )
        })?;
        self.data = &self.data[size..];
        Ok(result)
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Utility

/// Makes a panic info a little easier to read by splitting it into the message and location
pub fn panic_message_split_to_message_and_location(
    panic_info: &std::panic::PanicInfo<'_>,
) -> (String, String) {
    let panic_info_content = format!("{}", panic_info).replace("panicked at '", "");
    if let Some(split_pos) = panic_info_content.rfind("', ") {
        let (message, location) = panic_info_content.split_at(split_pos);
        let location = location.replace("', ", "");
        (message.to_string(), location)
    } else {
        ("Panicked".to_string(), panic_info_content)
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Transmutation convenience functions

/// Helper function for when we need a additional reference of an object
/// IMPORTANT: This can be highly unsafe! So use sparingly!
pub fn transmute_to_additional_ref<Typename>(obj: &Typename) -> &'static Typename {
    unsafe { std::mem::transmute::<&Typename, &'static Typename>(obj) }
}

/// Helper function for when we need a additional mutable reference of an object
/// IMPORTANT: This can be highly unsafe! So use sparingly!
pub fn transmute_to_additional_ref_mut<Typename>(obj: &mut Typename) -> &'static mut Typename {
    unsafe { std::mem::transmute::<&mut Typename, &'static mut Typename>(obj) }
}

pub fn transmute_to_byte_slice<S>(from: &S) -> &[u8] {
    unsafe { std::slice::from_raw_parts((from as *const S) as *const u8, std::mem::size_of::<S>()) }
}

pub fn transmute_to_byte_slice_mut<S>(from: &mut S) -> &mut [u8] {
    unsafe { std::slice::from_raw_parts_mut((from as *mut S) as *mut u8, std::mem::size_of::<S>()) }
}

pub fn transmute_to_slice<S, D>(from: &S) -> &[D] {
    unsafe {
        std::slice::from_raw_parts(
            (from as *const S) as *const D,
            std::mem::size_of::<S>() / std::mem::size_of::<D>(),
        )
    }
}

pub fn transmute_to_slice_mut<S, D>(from: &mut S) -> &mut [D] {
    unsafe {
        std::slice::from_raw_parts_mut(
            (from as *mut S) as *mut D,
            std::mem::size_of::<S>() / std::mem::size_of::<D>(),
        )
    }
}

pub fn transmute_slice_to_byte_slice<S>(from: &[S]) -> &[u8] {
    unsafe {
        std::slice::from_raw_parts(
            from.as_ptr() as *const u8,
            from.len() * std::mem::size_of::<S>(),
        )
    }
}

pub fn transmute_slice_to_byte_slice_mut<S>(from: &mut [S]) -> &mut [u8] {
    unsafe {
        std::slice::from_raw_parts_mut(
            from.as_mut_ptr() as *mut u8,
            from.len() * std::mem::size_of::<S>(),
        )
    }
}

pub fn transmute_slices<S, D>(from: &[S]) -> &[D] {
    unsafe {
        std::slice::from_raw_parts(
            from.as_ptr() as *const D,
            from.len() * std::mem::size_of::<S>() / std::mem::size_of::<D>(),
        )
    }
}

pub fn transmute_slices_mut<S, D>(from: &mut [S]) -> &mut [D] {
    unsafe {
        std::slice::from_raw_parts_mut(
            from.as_mut_ptr() as *mut D,
            from.len() * std::mem::size_of::<S>() / std::mem::size_of::<D>(),
        )
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Finding in containers

#[inline]
pub fn slice_index_of_max<T: Ord>(slice: &[T]) -> Option<usize> {
    slice
        .iter()
        .enumerate()
        .max_by(|(_a_index, a_val), (_b_index, b_val)| a_val.cmp(b_val))
        .map(|(index, _value)| index)
}

#[inline]
pub fn slice_index_of_min<T: Ord>(slice: &[T]) -> Option<usize> {
    slice
        .iter()
        .enumerate()
        .min_by(|(_a_index, a_val), (_b_index, b_val)| a_val.cmp(b_val))
        .map(|(index, _value)| index)
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Hashing

/// Hashes the input block using the FNV-1a hashfunction.
/// (https://en.wikipedia.org/wiki/Fowler%E2%80%93Noll%E2%80%93Vo_hash_function)
///
pub fn hash_string_64(input: &str) -> u64 {
    const FNV_PRIME: u64 = 1099511628211;
    const FNV_OFFSET_BASIS: u64 = 14695981039346656037;

    let mut hash = std::num::Wrapping(FNV_OFFSET_BASIS);
    let prime = std::num::Wrapping(FNV_PRIME);
    for byte in input.bytes() {
        hash.0 ^= byte as u64;
        hash *= prime;
    }
    hash.0
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Finding all common divisors of a given list (useful for finding scaled down display resolutions)

pub fn get_all_divisors(value: u32) -> Vec<u32> {
    (1..(value / 2)).filter(|x| value % x == 0).collect()
}

pub fn common_divisors(values: &[u32]) -> Vec<u32> {
    use std::collections::HashSet;
    let mut divisor_sets: Vec<HashSet<u32>> = values
        .iter()
        .map(|value| get_all_divisors(*value).into_iter().collect())
        .collect();

    let initial_set = divisor_sets.pop().unwrap();
    let intersection_set: HashSet<u32> =
        divisor_sets.into_iter().fold(initial_set, |acc, other| {
            HashSet::intersection(&acc, &other).cloned().collect()
        });

    let mut result: Vec<u32> = intersection_set.into_iter().collect();
    result.sort();
    result
}

pub fn get_all_resolution_divisors(resolution: (u32, u32)) -> Vec<(u32, u32)> {
    common_divisors(&[resolution.0, resolution.1])
        .iter()
        .map(|divisor| (resolution.0 / divisor, resolution.1 / divisor))
        .collect()
}

pub fn common_resolutions(resolutions: &[(u32, u32)]) -> Vec<(u32, u32)> {
    use std::collections::HashSet;
    let mut divisor_sets: Vec<HashSet<(u32, u32)>> = resolutions
        .iter()
        .map(|value| get_all_resolution_divisors(*value).into_iter().collect())
        .collect();

    let initial_set = divisor_sets.pop().unwrap();
    let intersection_set: HashSet<(u32, u32)> =
        divisor_sets.into_iter().fold(initial_set, |acc, other| {
            HashSet::intersection(&acc, &other).cloned().collect()
        });

    let mut result: Vec<(u32, u32)> = intersection_set.into_iter().collect();
    result.sort();
    result
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Min/Max/Sort floats

pub fn min_in_list_f32(values: &[f32]) -> f32 {
    values
        .iter()
        .fold(std::f32::MAX, |acc, val| f32::min(acc, *val))
}

pub fn max_in_list_f32(values: &[f32]) -> f32 {
    values
        .iter()
        .fold(std::f32::MIN, |acc, val| f32::max(acc, *val))
}

pub fn min_in_list_f64(values: &[f64]) -> f64 {
    values
        .iter()
        .fold(std::f64::MAX, |acc, val| f64::min(acc, *val))
}

pub fn max_in_list_f64(values: &[f64]) -> f64 {
    values
        .iter()
        .fold(std::f64::MIN, |acc, val| f64::max(acc, *val))
}

pub fn sort_list_f32(values: &mut [f32]) {
    values.sort_by(|a, b| {
        a.partial_cmp(b)
            .unwrap_or_else(|| std::cmp::Ordering::Equal)
    })
}

pub fn sort_list_f64(values: &mut [f64]) {
    values.sort_by(|a, b| {
        a.partial_cmp(b)
            .unwrap_or_else(|| std::cmp::Ordering::Equal)
    })
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Useful code snippets

// #[rustfmt::skip]
