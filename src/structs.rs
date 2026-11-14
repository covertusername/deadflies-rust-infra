use std::{ops::{Deref, DerefMut, Range}, ptr::addr_eq, slice::{Iter, IterMut}, sync::{Arc, Weak}};
use parking_lot::RwLock;
use crate::primitives::IType;

pub type ArcRw<T> = Arc<RwLock<T>>;

#[derive(Debug)]
pub struct StructItem<T, U, K, V>
where
    T: 'static,
    U: Ord + 'static,
    V: Clone,
{
    pub name: String,
    pub value: IType<T, U, K, V>,
}

impl<T, U, K, V> Clone for StructItem<T, U, K, V>
where
    T: 'static,
    U: Ord + 'static,
    V: Clone,
{
    fn clone(&self) -> Self {
        Self { name: self.name.clone(), value: self.value.clone() }
    }
}

#[derive(Debug)]
pub struct Struct<T, U, K, V>
where
    T: 'static,
    U: Ord + 'static,
    V: Clone,
{
    pub entries: Vec<StructItem<T, U, K, V>>,
}

impl<T, U, K, V> Clone for Struct<T, U, K, V>
where
    T: 'static,
    U: Ord + 'static,
    V: Clone,
{
    fn clone(&self) -> Self {
        Self { entries: self.entries.clone() }
    }
}

/*
 * i have made multiple attempts at a good, idiomatic implementation of
 * Iterator on Lists at this point. the first attempt was a blanket
 * implementation of Iterator on dyn ListCore<T>. this did not work well since
 * some list types (namely Stack and Queue) do not support "for each" within
 * Infra, so impelementing Iterator doesn't make sense for those types. i then
 * tried switching to implementing Iterator per-type, but i ran into an issue
 * iterating on a Set within Set::is_subset_of(). i would have had to implement
 * Iterator directly (not IntoIterator) which requires an adapter struct. i do
 * not know how to do this, and i don't really have the time to figure it out
 * right now. the Lists that do allow iteration (plan Lists and Sets) have an
 * iter() function defined that returns slice::Iter using items_ref() for now.
 * Lists additionally have iter_mut() defined similarly since they have no
 * additional semantics that must be upheld.
 * if you really need to iterate on Stacks or Queues, use make_vec().
 */

trait ListPriv<T>
where
    T: ?Sized + 'static,
    Self: 'static,
{
    //method currently going unused
    //fn items(self) -> Vec<ArcRw<T>>;

    fn items_ref(&self) -> &Vec<ArcRw<T>>;

    fn items_mut(&mut self) -> &mut Vec<ArcRw<T>>;
}

#[allow(private_bounds)]
pub trait ListCore<T>: ListPriv<T>
where
    T: ?Sized + 'static,
    Self: 'static,
{
    // useful for List types that don't directly implement IntoIterator
    fn make_vec(&mut self) -> Vec<ArcRw<T>>;

    fn append(&mut self, item: T) 
    where
        T: Sized,
    {
        self.items_mut().push(Arc::new(RwLock::new(item)));
    }

    fn append_arc(&mut self, item: ArcRw<T>) {
        self.items_mut().push(item.clone());
    }

    /*
     * standard says this is correct even though it violates for each
     * semantics on some list types of b
     */
    fn extend(&mut self, b: &dyn ListCore<T>) {
        for item in b.items_ref() {
            self.items_mut().push(item.clone());
        }
    }

    fn prepend(&mut self, item: T)
    where
        T: Sized,
    {
        self.items_mut().insert(0, Arc::new(RwLock::new(item)));
    }

    fn insert(&mut self, item: T, index: usize)
    where
        T: Sized,
    {
        self.items_mut().insert(index, Arc::new(RwLock::new(item)));
    }

    fn empty(&mut self) {
        self.items_mut().clear();
    }

    /*
     * i would have liked to use Vec::contains() but returning the index
     * of the first instance is useful.
     */
    #[must_use]
    fn contains(&self, item: &T) -> Option<usize>
    where
        T: PartialEq,
    {
        for (index, my_item) in self.items_ref().iter().enumerate() {
            if *item == *my_item.read_arc() {
                return Some(index);
            }
        }
        None
    }

    // useful when T is not PartialEq
    #[must_use]
    fn contains_arc(&self, item: &ArcRw<T>) -> Option<usize> {
        for (index, my_item) in self.items_ref().iter().enumerate() {
            if Arc::ptr_eq(item, my_item) {
                return Some(index);
            }
        }
        None
    }

    // useful for cyclic references (CyclicModuleRecords so far)
    #[must_use]
    fn contains_weak(&self, item: &Weak<RwLock<T>>) -> Option<usize> {
        for (index, my_item) in self.items_ref().iter().enumerate() {
            if addr_eq(item.as_ptr(), Arc::as_ptr(my_item)) {
                return Some(index);
            }
        }
        None

    }

    #[must_use]
    fn exists(&self, index: usize) -> bool {
        index < self.items_ref().len()
    }

    #[must_use]
    fn size(&self) -> usize {
        self.items_ref().len()
    }

    #[must_use]
    fn is_empty(&self) -> bool {
        self.size() == 0
    }

    #[must_use]
    fn get_indices(&self) -> Set<usize> {
        Set::from_range(0..self.size())
    }
    
    /*
     * Self must be Clone since the standard requires us to clone the
     * list. additionally, since these are in fact Clone, i don't
     * think it matters if we allow these to work on Stacks or Queues
     * since it's not changing the original list.
     */
    #[must_use]
    /*
     * there may be a good way to use sort_by_key but i do not know it.
     * clippy's suggested fix causes a move out of shared reference (since T
     * will escape the RwLock via the closure's return value)
     */
    #[allow(clippy::unnecessary_sort_by)]
    fn sort_ascend(&mut self) -> Box<dyn ListCore<T>>
    where
        T: Ord,
        Self: Clone,
    {
        let mut ret = Box::new(self.clone());

        ret.items_mut().sort_by(|a: &ArcRw<T>, b: &ArcRw<T>|
            (*a.read_arc()).cmp(&*b.read_arc())
        );
        ret
    }
    
    // Self must be Clone since the standard requires us to clone the list
    #[must_use]
    fn sort_descend(&mut self) -> Box<dyn ListCore<T>>
    where
        T: Ord,
        Self: Clone,
    {
        let mut ret = Box::new(self.clone());

        ret.items_mut().sort_by(|a: &ArcRw<T>, b: &ArcRw<T>|
            (*a.read_arc()).cmp(&*b.read_arc()).reverse()
        );
        ret
    }
}

pub trait ListExtra<T>: ListCore<T>
where
    T: 'static,
{
    fn replace<F>(&mut self, rep: T, mut f: F)
    where
        F: FnMut(&T) -> bool,
    {
        let wrapped = Arc::new(RwLock::new(rep));
        let items = self.items_mut();

        for item in items.iter_mut() {
            if f(&*item.read_arc()) {
                *item = wrapped.clone();
            }
        }
    }

    fn remove<F>(&mut self, mut f: F)
    where
        F: FnMut(&T) -> bool,
    {
        self.items_mut().retain(|x| !f(&*x.read_arc()));
    }
}

impl<T> ListExtra<T> for dyn ListCore<T>
where
    T: 'static
{}

#[derive(Debug)]
pub struct List<T>
where
    T: ?Sized,
{
    items: Vec<ArcRw<T>>,
}

impl<T> ListPriv<T> for List<T>
where
    T: ?Sized + 'static,
{
    fn items_ref(&self) -> &Vec<ArcRw<T>> {
        &self.items
    }

    fn items_mut(&mut self) -> &mut Vec<ArcRw<T>> {
        &mut self.items
    }
}

impl<T> ListCore<T> for List<T>
where
    T: ?Sized + 'static
{
    fn make_vec(&mut self) -> Vec<ArcRw<T>> {
        self.items_mut().clone()
    }
}

impl<T> ListExtra<T> for List<T>
where
    T: 'static,
{}

impl<T> List<T>
where
    T: ?Sized + 'static,
{
    #[must_use]
    pub fn new() -> Self {
        List { items: Vec::new() }
    }

    // makes certain operations easier elsewhere
    pub fn extend_vec(&mut self, vec: Vec<T>)
    where
        T: Sized,
    {
        for item in vec {
            self.append(item);
        }
    }

    // ran into lifetime issues when implementing this, so ignoring
    #[allow(clippy::iter_without_into_iter)]
    pub fn iter(&self) -> Iter<'_, ArcRw<T>> {
        self.items_ref().iter()
    }

    // ran into lifetime issues when implementing this, so ignoring
    #[allow(clippy::iter_without_into_iter)]
    pub fn iter_mut(&mut self) -> IterMut<'_, ArcRw<T>> {
        self.items_mut().iter_mut()
    }
}

impl<T> Default for List<T>
where
    T: 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

// derive macro no longer works. see Set's Clone impl for details
impl<T> Clone for List<T>
where
    T: 'static,
{
    fn clone(&self) -> Self {
        Self { items: self.items_ref().clone() }
    }
}

/*
 * for convenience within js_base, makes sublist/subslice creation easier.
 * remember to import ListCore to use List::append (otherwise Rust will try
 * to use Vec::append and you will get strange errors)
 */
impl<T> Deref for List<T>
where
    T: ?Sized + 'static,
{
    type Target = Vec<ArcRw<T>>;

    fn deref(&self) -> &Self::Target {
        &self.items
    }
}

impl<T> DerefMut for List<T>
where
    T: ?Sized + 'static,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.items
    }
}

#[derive(Debug)]
pub struct Stack<T> {
    items: Vec<ArcRw<T>>,
}

impl<T> ListPriv<T> for Stack<T>
where
    T: 'static,
{
    fn items_ref(&self) -> &Vec<ArcRw<T>> {
        &self.items
    }

    fn items_mut(&mut self) -> &mut Vec<ArcRw<T>> {
        &mut self.items
    }
}

impl<T> ListCore<T> for Stack<T>
where
    T: 'static
{
    fn make_vec(&mut self) -> Vec<ArcRw<T>> {
        let mut ret = Vec::new();

        while !self.is_empty() {
            ret.push(self.pop().unwrap());
        }
        ret
    }

    fn append(&mut self, _item: T) {
        eprintln!("warning: append() not supported on stacks");
    }

    fn append_arc(&mut self, _item: ArcRw<T>) {
        eprintln!("warning: append_arc() not supported on stacks");
    }

    fn prepend(&mut self, _item: T) {
        eprintln!("warning: prepend() not supported on stacks");
    }

    fn insert(&mut self, _item: T, _index: usize) {
        eprintln!("warning: insert() not supported on stacks");
    }
}

impl<T> ListExtra<T> for Stack<T>
where
    T: 'static
{
    fn replace<F>(&mut self, _rep: T, mut _f: F)
    where
        F: FnMut(&T) -> bool,
    {
        eprintln!("warning: replace() not supported on stacks");
    }

    fn remove<F>(&mut self, mut _f: F)
    where
        F: FnMut(&T) -> bool,
    {
        eprintln!("warning: remove() not supported on stacks");
    }
}

impl<T> Stack<T>
where
    T: 'static
{
    #[must_use]
    pub fn new() -> Self {
        Stack { items: Vec::new() }
    }

    pub fn push(&mut self, item: T) {
        self.items_mut().push(Arc::new(RwLock::new(item)));
    }

    pub fn pop(&mut self) -> Option<ArcRw<T>> {
        self.items_mut().pop()
    }

    #[must_use]
    pub fn peek(&self) -> Option<ArcRw<T>> {
        if self.is_empty() {
            return None;
        }
        Some(self.items_ref()[self.size() - 1].clone())
    }
}

impl<T> Default for Stack<T>
where
    T: 'static
{
    fn default() -> Self {
        Self::new()
    }
}

// derive macro no longer works. see Set's Clone impl for details
impl<T> Clone for Stack<T>
where
    T: 'static,
{
    fn clone(&self) -> Self {
        Self { items: self.items_ref().clone() }
    }
}

/*
 * may replace the Vec with a VecDeque for performance reasons but not
 * sure how to make it work with ListCore
 */
#[derive(Debug)]
pub struct Queue<T> {
    items: Vec<ArcRw<T>>,
}

impl<T> ListPriv<T> for Queue<T>
where
    T: 'static,
{
    fn items_ref(&self) -> &Vec<ArcRw<T>> {
        &self.items
    }

    fn items_mut(&mut self) -> &mut Vec<ArcRw<T>> {
        &mut self.items
    }
}

impl<T> ListCore<T> for Queue<T>
where
    T: 'static
{
    fn make_vec(&mut self) -> Vec<ArcRw<T>> {
        let mut ret = Vec::new();

        while !self.is_empty() {
            ret.push(self.dequeue().unwrap());
        }
        ret
    }
    
    fn append(&mut self, _item: T) {
        eprintln!("warning: append() not supported on queues");
    }

    fn append_arc(&mut self, _item: ArcRw<T>) {
        eprintln!("warning: append_arc() not supported on queues");
    }
    
    fn prepend(&mut self, _item: T) {
        eprintln!("warning: prepend() not supported on queues");
    }
    
    fn insert(&mut self, _item: T, _index: usize) {
        eprintln!("warning: insert() not supported on queues");
    }
}

impl<T> ListExtra<T> for Queue<T>
where
    T: 'static
{
    fn replace<F>(&mut self, _rep: T, mut _f: F)
    where
        F: FnMut(&T) -> bool,
    {
        eprintln!("warning: replace() not supported on queues");
    }

    fn remove<F>(&mut self, mut _f: F)
    where
        F: FnMut(&T) -> bool,
    {
        eprintln!("warning: remove() not supported on queues");
    }
}

impl<T> Queue<T>
where
    T: 'static
{
    #[must_use]
    pub fn new() -> Self {
        Queue { items: Vec::new() }
    }

    pub fn enqueue(&mut self, item: T) {
        self.items_mut().push(Arc::new(RwLock::new(item)));
    }

    pub fn dequeue(&mut self) -> Option<ArcRw<T>> {
        if self.is_empty() {
            return None;
        }
        Some(self.items_mut().remove(0))
    }
}

impl<T> Default for Queue<T>
where
    T: 'static
{
    fn default() -> Self {
        Self::new()
    }
}

// derive macro no longer works. see Set's Clone impl for details
impl<T> Clone for Queue<T>
where
    T: 'static,
{
    fn clone(&self) -> Self {
        Self { items: self.items_ref().clone() }
    }
}

#[derive(Debug)]
pub struct Set<T>
where
    T: Ord,
{
    items: Vec<ArcRw<T>>,
}

impl<T> ListPriv<T> for Set<T>
where
    T: Ord + 'static,
{
    fn items_ref(&self) -> &Vec<ArcRw<T>> {
        &self.items
    }

    fn items_mut(&mut self) -> &mut Vec<ArcRw<T>> {
        &mut self.items
    }
}

impl<T> ListCore<T> for Set<T>
where
    T: Ord + 'static
{
    fn make_vec(&mut self) -> Vec<ArcRw<T>> {
        self.items_mut().clone()
    }
    
    fn append(&mut self, item: T) {
        if self.contains(&item).is_none() {
            self.items_mut().push(Arc::new(RwLock::new(item)));
        }
    }

    fn append_arc(&mut self, item: ArcRw<T>) {
        if self.contains_arc(&item).is_none() {
            self.items_mut().push(item.clone());
        }
    }
    
    fn extend(&mut self, b: &dyn ListCore<T>) {
        for item in b.items_ref() {
            self.append_arc(item.clone());
        }
    }
    
    fn prepend(&mut self, item: T) {
        if self.contains(&item).is_none() {
            self.items_mut().insert(0, Arc::new(RwLock::new(item)));
        }
    }
    
    fn insert(&mut self, item: T, index: usize) {
        if self.contains(&item).is_none() {
            self.items_mut().insert(index, Arc::new(RwLock::new(item)));
        }
    }
}

impl<T> ListExtra<T> for Set<T>
where
    T: Ord + 'static,
{
    /*
     * replace as defined for Sets effectively has a different type signature.
     * since Rust doesn't support function overloading, we instead simulate the
     * function by using the first match of |x| x == item
     */
    fn replace<F>(&mut self, rep: T, mut f: F)
    where
        F: FnMut(&T) -> bool,
    {
        for (ii, item) in self.items_mut().iter().enumerate() {
            if !f(&*item.read_arc()) {
                continue;
            }
            match self.contains(&rep) {
                Some(ri) => match ri.cmp(&ii) {
                    std::cmp::Ordering::Less =>
                        drop(self.items_mut().remove(ii)),
                    std::cmp::Ordering::Equal => (),
                    std::cmp::Ordering::Greater => {
                        self.items_mut()[ii] = Arc::new(RwLock::new(rep));
                        self.items_mut().remove(ri);
                    },
                },
                None => self.items_mut()[ii] = Arc::new(RwLock::new(rep)),
            }
            return;
        }
    }
}

impl<T> Set<T>
where
    T: Ord + 'static,
{
    #[must_use]
    pub fn new() -> Self {
        Set { items: Vec::new() }
    }

    pub fn create(input: &mut dyn ListCore<T>) -> Self {
        let mut ret = Set::new();

        for item in input.make_vec() {
            ret.items_mut().push(item.clone());
        }
        ret
    }

    // ran into lifetime issues when implementing this, so ignoring
    #[allow(clippy::iter_without_into_iter)]
    pub fn iter(&self) -> Iter<'_, ArcRw<T>> {
        self.items_ref().iter()
    }

    #[must_use]
    // if we panic this is a bug
    #[allow(clippy::missing_panics_doc)]
    pub fn is_subset_of(&self, superset: &Self) -> bool {
        for item in self.iter() {
            if superset.contains(&*item.read_arc()).is_none() {
                return false
            }
        }
        true
    }

    #[must_use]
    pub fn is_superset_of(&self, subset: &Self) -> bool {
        subset.is_subset_of(self)
    }

    #[must_use]
    // if we panic this is a bug
    #[allow(clippy::missing_panics_doc)]
    pub fn intersection(&self, other: &Self) -> Self {
        let mut ret = Set::new();

        for item in self.iter() {
            if other.contains(&*item.read_arc()).is_some() {
                ret.append_arc(item.clone());
            }
        }
        ret
    }

    #[must_use]
    pub fn union(&self, other: &Self) -> Self {
        let mut ret = self.clone();

        for item in other.iter() {
            ret.append_arc(item.clone());
        }
        ret
    }

    #[must_use]
    // if we panic this is a bug, or other threads have panicked
    #[allow(clippy::missing_panics_doc)]
    pub fn difference(&self, other: &Self) -> Self {
        let mut ret = Set::new();

        for item in self.iter() {
            if other.contains(&*item.read_arc()).is_none() {
                ret.append_arc(item.clone());
            }
        }
        ret
    }
}

impl<T> Default for Set<T>
where
    T: Ord + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<T> PartialEq for Set<T>
where
    T: Ord + 'static,
{
    fn eq(&self, other: &Self) -> bool {
        self.is_subset_of(other) && self.is_superset_of(other)
    }
}

/*
 * the derive macro doesn't work anymore since we indirect T through the 
 * Arc - T doesn't need to be Clone, but the derive macro requires that it
 * is. also, i wish i could blanket impl this but there's no way to clone
 * a dyn since it doesn't have a known size yet. maybe i need to start
 * learning macros, but i'll worry about that if it comes up again. even
 * so, not sure how the where clause here would impact the macro
 */
impl<T> Clone for Set<T>
where
    T: Ord + 'static,
{
    fn clone(&self) -> Self {
        Self { items: self.items_ref().clone() }
    }
}

impl Set<usize> {
    /*
     * the infra standard defines two versions of "the range", with the ending
     * being either inclusive or exclusive. thankfully, we can sidestep that
     * using Rust's ranges, and just create the set from one
     */
    #[must_use]
    pub fn from_range(range: Range<usize>) -> Self {
        let mut ret = Self::new();

        for i in range {
            ret.append(i);
        }
        ret
    }
}

/*
 * i would love to use Rust's HashMap internally here but you can't sort it in
 * a decending manner, which the Infra Map requires, so i gotta roll my own
*/
#[derive(Debug)]
pub struct Map<K, V>
where
    V: Clone,
{
    /*
     * originally wanted the second item in the tuple to be ArcRw<V>, but
     * this caused problems when V is a trait object. if you need to use a
     * trait object (or some non-cloneable type) as V then explicitly wrap it
     * in a Box, ArcRw, or similar
     */
    items: Vec<(ArcRw<K>, V)>,
}

impl<K, V> Map<K, V>
where
    K: Ord + 'static,
    V: Clone + 'static,
{
    #[must_use]
    pub const fn new() -> Self {
        Self { items: Vec::new() }
    }

    #[must_use]
    // if we panic this is a bug, or other threads have panicked
    #[allow(clippy::missing_panics_doc)]
    pub fn contains_key(&self, key: &K) -> Option<usize>
    where
        K: PartialEq,
    {
        for (index, (my_key, _)) in self.items.iter().enumerate() {
            if *key == *my_key.read_arc() {
                return Some(index);
            }
        }
        None
    }

    #[must_use]
    // useful when K is not PartialEq, esp. in the case of Symbols
    pub fn contains_key_arc(&self, key: &ArcRw<K>) -> Option<usize> {
        for (index, (my_key, _)) in self.items.iter().enumerate() {
            if Arc::ptr_eq(key, my_key) {
                return Some(index);
            }
        }
        None
    }

    // the sane implementation
    #[must_use]
    pub fn get(&self, key: &K) -> Option<V>
    where
        K: PartialEq,
    {
        self.contains_key(key).map(|i| self.items[i].1.clone())
    }

    #[must_use]
    pub fn get_arc(&self, key: &ArcRw<K>) -> Option<V> {
        self.contains_key_arc(key).map(|i| self.items[i].1.clone())
    }

    // the insane Infra definition
    #[must_use]
    // if we panic this is a bug
    #[allow(clippy::missing_panics_doc)]
    pub fn get_or_d(&self, key: &K, d: Option<V>) -> V
    where
        K: PartialEq,
    {
        match self.get(key) {
            Some(v) => v,
            None => match d {
                Some(v) => v.clone(),
                None => panic!("couldn't find key, no default provided"),
            },
        }
    }

    // i really really hope i never need to use this
    #[must_use]
    // if we panic this is a bug
    #[allow(clippy::missing_panics_doc)]
    pub fn get_or_d_arc(&self, key: &ArcRw<K>, d: Option<V>) -> V {
        match self.get_arc(key) {
            Some(v) => v.clone(),
            None => match d {
                Some(v) => v,
                None => panic!("couldn't find key, no default provided"),
            },
        }
    }

    pub fn set(&mut self, key: K, value: V)
    where
        K: PartialEq,
    {
        match self.contains_key(&key) {
            Some(i) => self.items[i].1 = value,
            None => self.items.push((Arc::new(RwLock::new(key)), value)),
        }
    }

    pub fn set_arc(&mut self, key: ArcRw<K>, value: V) {
        match self.contains_key_arc(&key) {
            Some(i) => self.items[i].1 = value,
            None => self.items.push((key, value)),
        }
    }

    pub fn remove<F>(&mut self, mut f: F)
    where
        F: FnMut(&(ArcRw<K>, V)) -> bool,
    {
        self.items.retain(|e| !f(e));
    }

    pub fn clear(&mut self) {
        self.items.clear();
    }

    #[must_use]
    pub fn get_keys(&self) -> Set<K> {
        let mut ret = Set::new();

        for (k, _) in &self.items {
            ret.append_arc(k.clone());
        }
        ret
    }

    #[must_use]
    pub fn get_values(&self) -> List<V> {
        let mut ret = List::new();

        for (_, v) in &self.items {
            ret.append(v.clone());
        }
        ret
    }

    /*
     * technically this is the size of the set returned by get_keys but this
     * gives the same answer with simpler code (also the infra def is dumb)
     */
    #[must_use]
    pub fn size(&self) -> usize {
        self.items.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.size() == 0
    }

    #[must_use]
    // if we panic this is a bug
    #[allow(clippy::missing_panics_doc)]
    pub fn sort_ascend(&mut self) -> Self
    where
        K: Ord + Clone,
    {
        let mut ret = self.clone();

        ret.items.sort_by_key(|(a, _)|
            (*a.read_arc()).clone()
        );
        ret
    }
    
    #[must_use]
    // if we panic this is a bug, or other threads have panicked
    #[allow(clippy::missing_panics_doc)]
    pub fn sort_descend(&mut self) -> Self
    where
        K: Ord,
    {
        let mut ret = self.clone();

        ret.items.sort_by(|(a, _), (b, _)|
            (*a.read_arc()).cmp(&*b.read_arc()).reverse()
        );
        ret
    }
}

impl<K, V> Default for Map<K, V>
where
    K: Ord + 'static,
    V: Clone + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<K, V> IntoIterator for Map<K, V>
where
    V: Clone,
{
    type Item = (ArcRw<K>, V);

    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.items.into_iter()
    }
}

// derive doesn't work here for the same reasons as the Lists
impl<K, V> Clone for Map<K, V>
where
    V: Clone,
{
    fn clone(&self) -> Self {
        Self { items: self.items.clone() }
    }
}
