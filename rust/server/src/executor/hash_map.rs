use crate::array2::{Array, ArrayImpl, DataChunk};
use crate::error::Result;
use crate::types::Scalar;
use crate::types::{IntervalUnit, ScalarImpl, ScalarRef};
use crate::util::hash_util::CRC32FastBuilder;
use rust_decimal::Decimal;
use std::convert::TryInto;
use std::default::Default;
use std::hash::{BuildHasher, Hash, Hasher};

/// Max element count in [`FixedSizeKeyWithHashCode`]
const MAX_FIXED_SIZE_KEY_ELEMENTS: usize = 8;

pub trait HashKeySerializer {
    type K: HashKey;
    fn from_hash_code(hash_code: u64) -> Self;
    fn append<'a, D: HashKeySerialize<'a>>(&mut self, data: Option<D>) -> Result<()>;
    fn into_hash_key(self) -> Self::K;
}

pub trait HashKeySerialize<'a>: ScalarRef<'a> + Default {
    type S: AsRef<[u8]>;
    fn serialize(self) -> Self::S;
}

pub trait HashKey: Hash + Eq + Sized {
    type S: HashKeySerializer<K = Self>;

    fn build(column_idxes: &[usize], data_chunk: &DataChunk) -> Result<Vec<Self>> {
        let hash_codes = data_chunk.get_hash_values(column_idxes, CRC32FastBuilder)?;
        let mut serializers: Vec<Self::S> = hash_codes
            .into_iter()
            .map(Self::S::from_hash_code)
            .collect();

        for column_idx in column_idxes {
            data_chunk
                .column_at(*column_idx)?
                .array_ref()
                .serialize_to_hash_key(&mut serializers)?;
        }

        Ok(serializers
            .into_iter()
            .map(Self::S::into_hash_key)
            .collect())
    }
}

/// Designed for hash keys with at most `N` serialized bytes.
pub struct FixedSizeKey<const N: usize> {
    hash_code: u64,
    key: [u8; N],
    null_bitmap: u8,
}

/// Designed for hash keys which can't be represented by [`FixedSizeKey`].
///
/// When any of following conditions is met, we choose [`SerializedKey`]:
/// 1. Has variable size column.
/// 2. Number of columns exceeds [`MAX_FIXED_SIZE_KEY_ELEMENTS`]
/// 3. Sizes of data types exceed `256` bytes.
/// 4. Any column's serialized format can't be used for equality check.
pub struct SerializedKey {
    key: Vec<ScalarImpl>,
    null_bitmap: Vec<bool>,
    hash_code: u64,
}

/// Fix clippy warning.
impl<const N: usize> PartialEq for FixedSizeKey<N> {
    fn eq(&self, other: &Self) -> bool {
        (self.key == other.key) && (self.null_bitmap == other.null_bitmap)
    }
}

impl<const N: usize> Eq for FixedSizeKey<N> {}

impl<const N: usize> Hash for FixedSizeKey<N> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u64(self.hash_code)
    }
}

impl PartialEq for SerializedKey {
    fn eq(&self, other: &Self) -> bool {
        (self.null_bitmap == other.null_bitmap) && (self.key == other.key)
    }
}

impl Eq for SerializedKey {}

impl Hash for SerializedKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u64(self.hash_code)
    }
}

/// A special hasher designed our hashmap, which just stores precomputed hash key.
///
/// We need this because we compute hash keys in vectorized fashion, and we store them in this
/// hasher.
#[derive(Default)]
pub(in crate::executor) struct PrecomputedHasher {
    hash_code: u64,
}

impl Hasher for PrecomputedHasher {
    fn finish(&self) -> u64 {
        self.hash_code
    }

    fn write(&mut self, bytes: &[u8]) {
        self.hash_code = u64::from_ne_bytes(bytes.try_into().unwrap());
    }
}

#[derive(Default)]
pub(in crate::executor) struct PrecomputedBuildHasher;

impl BuildHasher for PrecomputedBuildHasher {
    type Hasher = PrecomputedHasher;

    fn build_hasher(&self) -> Self::Hasher {
        PrecomputedHasher::default()
    }
}

type Key16 = FixedSizeKey<2>;
type Key32 = FixedSizeKey<4>;
type Key64 = FixedSizeKey<8>;
type Key128 = FixedSizeKey<16>;
type Key256 = FixedSizeKey<32>;
type KeySerialized = SerializedKey;

impl HashKeySerialize<'_> for bool {
    type S = [u8; 1];

    fn serialize(self) -> Self::S {
        if self {
            [1u8; 1]
        } else {
            [0u8; 1]
        }
    }
}

impl HashKeySerialize<'_> for i16 {
    type S = [u8; 2];

    fn serialize(self) -> Self::S {
        self.to_ne_bytes()
    }
}

impl HashKeySerialize<'_> for i32 {
    type S = [u8; 4];

    fn serialize(self) -> Self::S {
        self.to_ne_bytes()
    }
}

impl HashKeySerialize<'_> for i64 {
    type S = [u8; 8];

    fn serialize(self) -> Self::S {
        self.to_ne_bytes()
    }
}

impl HashKeySerialize<'_> for f32 {
    type S = [u8; 4];

    fn serialize(self) -> Self::S {
        self.to_ne_bytes()
    }
}

impl HashKeySerialize<'_> for f64 {
    type S = [u8; 8];

    fn serialize(self) -> Self::S {
        self.to_ne_bytes()
    }
}

impl HashKeySerialize<'_> for Decimal {
    type S = [u8; 16];

    fn serialize(self) -> Self::S {
        Decimal::serialize(&self.normalize())
    }
}

impl HashKeySerialize<'_> for IntervalUnit {
    type S = [u8; 16];

    fn serialize(self) -> Self::S {
        let mut ret = [0; 16];
        ret[0..4].copy_from_slice(&self.get_months().to_ne_bytes());
        ret[4..8].copy_from_slice(&self.get_days().to_ne_bytes());
        ret[8..16].copy_from_slice(&self.get_ms().to_ne_bytes());

        ret
    }
}

impl<'a> HashKeySerialize<'a> for &'a str {
    type S = Vec<u8>;

    /// This should never be called
    fn serialize(self) -> Self::S {
        panic!("Should not serialize str for hash!")
    }
}

pub struct FixedSizeKeySerializer<const N: usize> {
    buffer: [u8; N],
    null_bitmap: u8,
    null_bitmap_idx: usize,
    data_len: usize,
    hash_code: u64,
}

impl<const N: usize> FixedSizeKeySerializer<N> {
    fn left_size(&self) -> usize {
        N - self.data_len
    }
}

impl<const N: usize> HashKeySerializer for FixedSizeKeySerializer<N> {
    type K = FixedSizeKey<N>;

    fn from_hash_code(hash_code: u64) -> Self {
        Self {
            buffer: [0u8; N],
            null_bitmap: 0u8,
            null_bitmap_idx: 0,
            data_len: 0,
            hash_code,
        }
    }

    fn append<'a, D: HashKeySerialize<'a>>(&mut self, data: Option<D>) -> Result<()> {
        ensure!(self.null_bitmap_idx < 8);
        let data = match data {
            Some(v) => {
                let mask = 1u8 << self.null_bitmap_idx;
                self.null_bitmap |= mask;
                v.serialize()
            }
            None => {
                let mask = !(1u8 << self.null_bitmap_idx);
                self.null_bitmap &= mask;
                D::default().serialize()
            }
        };

        self.null_bitmap_idx += 1;

        let ret = data.as_ref();
        ensure!(self.left_size() >= ret.len());
        self.buffer[self.data_len..(self.data_len + ret.len())].copy_from_slice(ret);
        self.data_len += ret.len();
        Ok(())
    }

    fn into_hash_key(self) -> Self::K {
        FixedSizeKey::<N> {
            hash_code: self.hash_code,
            key: self.buffer,
            null_bitmap: self.null_bitmap,
        }
    }
}

pub struct SerializedKeySerializer {
    buffer: Vec<ScalarImpl>,
    null_bitmap: Vec<bool>,
    hash_code: u64,
}

impl HashKeySerializer for SerializedKeySerializer {
    type K = SerializedKey;

    fn from_hash_code(hash_code: u64) -> Self {
        Self {
            buffer: Vec::new(),
            null_bitmap: Vec::new(),
            hash_code,
        }
    }

    fn append<'a, D: HashKeySerialize<'a>>(&mut self, data: Option<D>) -> Result<()> {
        match data {
            Some(v) => {
                self.buffer.push(v.to_owned_scalar().to_scalar_value());
                self.null_bitmap.push(true);
            }
            None => {
                self.buffer
                    .push(D::default().to_owned_scalar().to_scalar_value());
                self.null_bitmap.push(false);
            }
        }
        Ok(())
    }

    fn into_hash_key(self) -> SerializedKey {
        SerializedKey {
            key: self.buffer,
            null_bitmap: self.null_bitmap,
            hash_code: self.hash_code,
        }
    }
}

fn serialize_array_to_hash_key<'a, A, S>(array: &'a A, serializers: &mut Vec<S>) -> Result<()>
where
    A: Array,
    A::RefItem<'a>: HashKeySerialize<'a>,
    S: HashKeySerializer,
{
    for (item, serializer) in array.iter().zip(serializers.iter_mut()) {
        serializer.append(item)?;
    }
    Ok(())
}

impl ArrayImpl {
    fn serialize_to_hash_key<S: HashKeySerializer>(&self, serializers: &mut Vec<S>) -> Result<()> {
        macro_rules! impl_all_serialize_to_hash_key {
      ([$self:ident, $serializers: ident], $({ $variant_name:ident, $suffix_name:ident, $array:ty, $builder:ty } ),*) => {
        match $self {
          $( Self::$variant_name(inner) => serialize_array_to_hash_key(inner, $serializers), )*
        }
      };
    }
        for_all_variants! { impl_all_serialize_to_hash_key, self, serializers }
    }
}

impl<const N: usize> HashKey for FixedSizeKey<N> {
    type S = FixedSizeKeySerializer<N>;
}

impl HashKey for SerializedKey {
    type S = SerializedKeySerializer;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::array;
    use crate::array2::column::Column;
    use crate::array2::{
        ArrayRef, BoolArray, DataChunk, DecimalArray, F32Array, F64Array, I16Array, I32Array,
        I64Array, UTF8Array,
    };
    use crate::executor::hash_map::{
        HashKey, Key128, Key16, Key256, Key32, Key64, KeySerialized, PrecomputedBuildHasher,
    };
    use crate::test_utils::rand_array::seed_rand_array_ref;
    use crate::types::{
        BoolType, DataTypeKind, Datum, DecimalType, Float32Type, Float64Type, Int16Type, Int32Type,
        Int64Type, StringType,
    };
    use std::collections::HashMap;
    use std::convert::TryFrom;
    use std::str::FromStr;
    use std::sync::Arc;

    #[derive(Hash, PartialEq, Eq)]
    struct Row(Vec<Datum>);

    fn generate_random_data_chunk() -> DataChunk {
        let capacity = 128;
        let seed = 10244021u64;
        let columns = vec![
            Column::new(
                seed_rand_array_ref::<BoolArray>(capacity, seed),
                BoolType::create(true),
            ),
            Column::new(
                seed_rand_array_ref::<I16Array>(capacity, seed),
                Int16Type::create(true),
            ),
            Column::new(
                seed_rand_array_ref::<I32Array>(capacity, seed),
                Int32Type::create(true),
            ),
            Column::new(
                seed_rand_array_ref::<I64Array>(capacity, seed),
                Int64Type::create(true),
            ),
            Column::new(
                seed_rand_array_ref::<F32Array>(capacity, seed),
                Float32Type::create(true),
            ),
            Column::new(
                seed_rand_array_ref::<F64Array>(capacity, seed),
                Float64Type::create(true),
            ),
            Column::new(
                seed_rand_array_ref::<DecimalArray>(capacity, seed),
                Arc::new(DecimalType::new(true, 20, 10).expect("Failed to create decimal type")),
            ),
            Column::new(
                seed_rand_array_ref::<UTF8Array>(capacity, seed),
                StringType::create(true, 20, DataTypeKind::Varchar),
            ),
        ];

        DataChunk::try_from(columns).expect("Failed to create data chunk")
    }

    fn do_test<K: HashKey, F>(column_indexes: Vec<usize>, data_gen: F)
    where
        F: FnOnce() -> DataChunk,
    {
        let data = data_gen();

        let mut actual_row_id_mapping = HashMap::<usize, Vec<usize>>::with_capacity(100);
        {
            let mut fast_hash_map =
                HashMap::<K, Vec<usize>, PrecomputedBuildHasher>::with_capacity_and_hasher(
                    100,
                    PrecomputedBuildHasher,
                );
            let keys =
                K::build(column_indexes.as_slice(), &data).expect("Failed to build hash keys");

            for (row_id, key) in keys.into_iter().enumerate() {
                let row_ids = fast_hash_map.entry(key).or_default();
                row_ids.push(row_id);
            }

            for row_ids in fast_hash_map.values() {
                for row_id in row_ids {
                    actual_row_id_mapping.insert(*row_id, row_ids.clone());
                }
            }
        }

        let mut expected_row_id_mapping = HashMap::<usize, Vec<usize>>::with_capacity(100);
        {
            let mut normal_hash_map: HashMap<Row, Vec<usize>> = HashMap::with_capacity(100);
            for row_idx in 0..data.capacity() {
                let row = column_indexes
                    .iter()
                    .map(|col_idx| {
                        data.column_at(*col_idx)
                            .unwrap_or_else(|_| panic!("Column not found: {}", *col_idx))
                    })
                    .map(|col| col.array_ref().scalar_value_at(row_idx))
                    .collect::<Vec<Datum>>();

                normal_hash_map.entry(Row(row)).or_default().push(row_idx);
            }

            for row_ids in normal_hash_map.values() {
                for row_id in row_ids {
                    expected_row_id_mapping.insert(*row_id, row_ids.clone());
                }
            }
        }

        assert_eq!(expected_row_id_mapping, actual_row_id_mapping);
    }

    #[test]
    fn test_two_bytes_hash_key() {
        do_test::<Key16, _>(vec![1], generate_random_data_chunk);
    }

    #[test]
    fn test_four_bytes_hash_key() {
        do_test::<Key32, _>(vec![0, 1], generate_random_data_chunk);
        do_test::<Key32, _>(vec![2], generate_random_data_chunk);
        do_test::<Key32, _>(vec![4], generate_random_data_chunk);
    }

    #[test]
    fn test_eight_bytes_hash_key() {
        do_test::<Key64, _>(vec![1, 2], generate_random_data_chunk);
        do_test::<Key64, _>(vec![0, 1, 2], generate_random_data_chunk);
        do_test::<Key64, _>(vec![3], generate_random_data_chunk);
        do_test::<Key64, _>(vec![5], generate_random_data_chunk);
    }

    #[test]
    fn test_128_bits_hash_key() {
        do_test::<Key128, _>(vec![3, 5], generate_random_data_chunk);
        do_test::<Key128, _>(vec![6], generate_random_data_chunk);
    }

    #[test]
    fn test_256_bits_hash_key() {
        do_test::<Key256, _>(vec![3, 5, 6], generate_random_data_chunk);
        do_test::<Key256, _>(vec![3, 6], generate_random_data_chunk);
    }

    #[test]
    fn test_var_length_hash_key() {
        do_test::<KeySerialized, _>(vec![0, 7], generate_random_data_chunk);
    }

    fn generate_decimal_test_data() -> DataChunk {
        let columns = vec![Column::new(
            Arc::new(
                array! { DecimalArray,
                  [Some(Decimal::from_str("1.2").unwrap()),
                   None,
                   Some(Decimal::from_str("1.200").unwrap()),
                   Some(Decimal::from_str("0.00").unwrap()),
                   Some(Decimal::from_str("0.0").unwrap())]
                }
                .into(),
            ) as ArrayRef,
            Arc::new(DecimalType::new(true, 20, 10).expect("Failed to create decimal type")),
        )];

        DataChunk::try_from(columns).expect("Failed to create data chunk")
    }

    #[test]
    fn test_decimal_hash_key_serialization() {
        do_test::<Key128, _>(vec![0], generate_decimal_test_data);
    }
}
