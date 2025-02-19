//! Various implementations of Domain.
//!
//! These different versions of [`Domain`] provide a general set of models used throughout OpenDP.
//! Most of the implementations are generic, with the type parameter setting the underlying [`Domain::Carrier`]
//! type.

use std::any::Any;
use std::collections::HashMap;
use std::hash::Hash;
use std::marker::PhantomData;
use std::ops::Bound;

use crate::core::Domain;
use crate::error::Fallible;

/// A Domain that contains all members of the carrier type.
pub struct AllDomain<T> {
    _marker: PhantomData<T>,
}
impl<T> AllDomain<T> {
    pub fn new() -> Self {
        AllDomain { _marker: PhantomData }
    }
}
// Auto-deriving Clone would put the same trait bound on T, so we implement it manually.
impl<T> Clone for AllDomain<T> {
    fn clone(&self) -> Self { Self::new() }
}
// Auto-deriving PartialEq would put the same trait bound on T, so we implement it manually.
impl<T> PartialEq for AllDomain<T> {
    fn eq(&self, _other: &Self) -> bool { true }
}
impl<T> Domain for AllDomain<T> {
    type Carrier = T;
    fn member(&self, _val: &Self::Carrier) -> bool { true }
}


/// A Domain that carries an underlying Domain in a Box.
#[derive(Clone, PartialEq)]
pub struct BoxDomain<D: Domain> {
    element_domain: Box<D>
}
impl<D: Domain> BoxDomain<D> {
    pub fn new(element_domain: Box<D>) -> Self {
        BoxDomain { element_domain }
    }
}
impl<D: Domain> Domain for BoxDomain<D> {
    type Carrier = Box<D::Carrier>;
    fn member(&self, val: &Self::Carrier) -> bool {
        self.element_domain.member(val)
    }
}


/// A Domain that unwraps a Data wrapper.
#[derive(Clone, PartialEq)]
pub struct DataDomain<D: Domain> {
    pub form_domain: D,
}
impl<D: Domain> DataDomain<D> {
    pub fn new(form_domain: D) -> Self {
        DataDomain { form_domain }
    }
}
impl<D: Domain> Domain for DataDomain<D> where
    D::Carrier: 'static + Any {
    type Carrier = Box<dyn Any>;
    fn member(&self, val: &Self::Carrier) -> bool {
        val.downcast_ref::<D::Carrier>()
            .map(|v| self.form_domain.member(v))
            .unwrap_or(false)
    }
}


/// A Domain that contains all the values in an interval.
#[derive(Clone, PartialEq)]
pub struct IntervalDomain<T> {
    lower: Bound<T>,
    upper: Bound<T>,
}
impl<T: PartialOrd> IntervalDomain<T> {
    pub fn new(lower: Bound<T>, upper: Bound<T>) -> Fallible<Self> {
        fn get<T>(value: &Bound<T>) -> Option<&T> {
            match value {
                Bound::Included(value) => Some(value),
                Bound::Excluded(value) => Some(value),
                Bound::Unbounded => None
            }
        }
        if let Some((v_lower, v_upper)) = get(&lower).zip(get(&upper)) {
            if v_lower > v_upper {
                return fallible!(MakeTransformation, "lower bound may not be greater than upper bound")
            }
            if v_lower == v_upper {
                match (&lower, &upper) {
                    (Bound::Included(_l), Bound::Excluded(_u)) =>
                        return fallible!(MakeTransformation, "upper bound excludes inclusive lower bound"),
                    (Bound::Excluded(_l), Bound::Included(_u)) =>
                        return fallible!(MakeTransformation, "lower bound excludes inclusive upper bound"),
                    _ => ()
                }
            }
        }
        Ok(IntervalDomain { lower, upper })
    }
}
impl<T: Clone + PartialOrd> Domain for IntervalDomain<T> {
    type Carrier = T;
    fn member(&self, val: &Self::Carrier) -> bool {
        let lower_ok = match &self.lower {
            Bound::Included(bound) => { val >= bound }
            Bound::Excluded(bound) => { val > bound }
            Bound::Unbounded => { true }
        };
        lower_ok && match &self.upper {
            Bound::Included(bound) => { val <= bound }
            Bound::Excluded(bound) => { val < bound }
            Bound::Unbounded => { true }
        }
    }
}


/// A Domain that contains pairs of values.
#[derive(Clone, PartialEq)]
pub struct PairDomain<D0: Domain, D1: Domain>(pub D0, pub D1);
impl<D0: Domain, D1: Domain> PairDomain<D0, D1> {
    pub fn new(element_domain0: D0, element_domain1: D1) -> Self {
        PairDomain(element_domain0, element_domain1)
    }
}
impl<D0: Domain, D1: Domain> Domain for PairDomain<D0, D1> {
    type Carrier = (D0::Carrier, D1::Carrier);
    fn member(&self, val: &Self::Carrier) -> bool {
        self.0.member(&val.0) && self.1.member(&val.1)
    }
}


/// A Domain that contains maps of (homogeneous) values.
#[derive(Clone, PartialEq)]
pub struct MapDomain<DK: Domain, DV: Domain> where DK::Carrier: Eq + Hash {
    pub key_domain: DK,
    pub value_domain: DV
}
impl<DK: Domain, DV: Domain> MapDomain<DK, DV> where DK::Carrier: Eq + Hash {
    pub fn new(key_domain: DK, element_domain: DV) -> Self {
        MapDomain { key_domain, value_domain: element_domain }
    }
}
impl<K, V> MapDomain<AllDomain<K>, AllDomain<V>> where K: Eq + Hash {
    pub fn new_all() -> Self {
        Self::new(AllDomain::<K>::new(), AllDomain::<V>::new())
    }
}
impl<DK: Domain, DV: Domain> Domain for MapDomain<DK, DV> where DK::Carrier: Eq + Hash {
    type Carrier = HashMap<DK::Carrier, DV::Carrier>;
    fn member(&self, val: &Self::Carrier) -> bool {
        val.iter().all(|(k, v)|
            self.key_domain.member( k) && self.value_domain.member(v))
    }
}


/// A Domain that contains vectors of (homogeneous) values.
#[derive(Clone, PartialEq)]
pub struct VectorDomain<D: Domain> {
    pub element_domain: D,
}
impl<D: Domain> VectorDomain<D> {
    pub fn new(element_domain: D) -> Self {
        VectorDomain { element_domain }
    }
}
impl<T> VectorDomain<AllDomain<T>> {
    pub fn new_all() -> Self {
        Self::new(AllDomain::<T>::new())
    }
}
impl<D: Domain> Domain for VectorDomain<D> {
    type Carrier = Vec<D::Carrier>;
    fn member(&self, val: &Self::Carrier) -> bool {
        val.iter().all(|e| self.element_domain.member(e))
    }
}

/// A Domain that specifies the length of the enclosed domain
#[derive(Clone, PartialEq)]
pub struct SizedDomain<D: Domain> {
    pub element_domain: D,
    pub length: usize
}
impl<D: Domain> SizedDomain<D> {
    pub fn new(member_domain: D, length: usize) -> Self {
        SizedDomain { element_domain: member_domain, length }
    }
}
impl<D: Domain> Domain for SizedDomain<D> {
    type Carrier = D::Carrier;
    fn member(&self, val: &Self::Carrier) -> bool {
        self.element_domain.member(val)
    }
}

/// A domain with a built-in representation of nullity, that may take on null values at runtime
#[derive(Clone, PartialEq)]
pub struct InherentNullDomain<D: Domain>
    where D::Carrier: InherentNull {
    pub element_domain: D,
}
impl<D: Domain> InherentNullDomain<D> where D::Carrier: InherentNull {
    pub fn new(member_domain: D) -> Self {
        InherentNullDomain { element_domain: member_domain }
    }
}
impl<D: Domain> Domain for InherentNullDomain<D> where D::Carrier: InherentNull {
    type Carrier = D::Carrier;
    fn member(&self, val: &Self::Carrier) -> bool {
        if val.is_null() {return true}
        self.element_domain.member(val)
    }
}
pub trait InherentNull {
    fn is_null(&self) -> bool;
    const NULL: Self;
}
macro_rules! impl_inherent_null_float {
    ($($ty:ty),+) => ($(impl InherentNull for $ty {
        #[inline]
        fn is_null(&self) -> bool { self.is_nan() }
        const NULL: Self = Self::NAN;
    })+)
}
impl_inherent_null_float!(f64, f32);

/// A domain that represents nullity via the Option type.
/// The value inside is non-null by definition.
/// Transformations should not emit data that can take on null-values at runtime.
/// For example, it is fine to have an OptionDomain<AllDomain<f64>>, but the f64 should never be nan
#[derive(Clone, PartialEq)]
pub struct OptionNullDomain<D: Domain> {
    pub element_domain: D,
}
impl<D: Domain> OptionNullDomain<D> {
    pub fn new(member_domain: D) -> Self {
        OptionNullDomain { element_domain: member_domain }
    }
}
impl<D: Domain> Domain for OptionNullDomain<D> {
    type Carrier = Option<D::Carrier>;
    fn member(&self, value: &Self::Carrier) -> bool {
        value.as_ref()
            .map(|v| self.element_domain.member(v))
            .unwrap_or(true)
    }
}
