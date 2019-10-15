use super::*;
use std::slice::IterMut;

#[derive(Debug)]
pub struct ChannelMap<T> {
    capacity: usize,
    elements: Vec<Slot<T>>,
}

#[derive(Debug)]
pub enum Slot<T> {
    Empty,
    Alloced,
    Filled(T),
}

impl<T> ChannelMap<T> {
    pub fn new(capacity: u32) -> Self {
        ChannelMap {
            capacity: capacity as usize,
            elements: Vec::with_capacity(1),
        }
    }

    pub fn len(&self) -> usize {
        let mut len = 0;
        for e in &self.elements {
            match e {
                Slot::Empty => (),
                _ => len += 1,
            }
        }
        len
    }

    pub fn get(&mut self, id: u32) -> Result<&mut T, ConnectionError> {
        match self.elements.get_mut(id as usize) {
            Some(Slot::Filled(t)) => Ok(t),
            _ => Err(ConnectionError::ChannelIdInvalid),
        }
    }

    pub fn alloc(&mut self) -> Option<u32> {
        for (i, e) in self.elements.iter().enumerate() {
            match e {
                Slot::Empty => {
                    self.elements[i] = Slot::Alloced;
                    return Some(i as u32)
                },
                _ => (),
            }
        }
        if self.elements.len() < self.capacity {
            let id = self.elements.len();
            self.elements.push(Slot::Alloced);
            return Some(id as u32);
        }
        None
    }

    pub fn insert(&mut self, id: u32, t: T) -> Result<(), ConnectionError> {
        if (id as usize) < self.elements.len() {
            match self.elements[id as usize] {
                Slot::Alloced => {
                    self.elements[id as usize] = Slot::Filled(t);
                    return Ok(())
                }
                _ => ()
            }
        }
        Err(ConnectionError::ChannelIdInvalid)
    }

    pub fn remove(&mut self, id: u32) -> Result<(), ConnectionError> {
        if (id as usize) < self.elements.len() {
            match self.elements[id as usize] {
                Slot::Filled(_) => {
                    self.elements[id as usize] = Slot::Empty;
                    return Ok(())
                }
                Slot::Alloced => {
                    self.elements[id as usize] = Slot::Empty;
                    return Ok(())
                }
                _ => ()
            }
        }
        Err(ConnectionError::ChannelIdInvalid)
    }

    pub fn iter<'a>(&'a mut self) -> ChannelMapIterator<'a, T> {
        ChannelMapIterator(self.elements.iter_mut())
    }
}

pub struct ChannelMapIterator<'a, T>(IterMut<'a, Slot<T>>);

impl<'a, T> Iterator for ChannelMapIterator<'a, T> {
    type Item = &'a mut T;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.0.next() {
                None => return None,
                Some(Slot::Empty) => continue,
                Some(Slot::Alloced) => continue, // skip
                Some(Slot::Filled(t)) => return Some(t),
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_new_01() {
        let m = ChannelMap::<()>::new(23);
        assert_eq!(m.capacity, 23);
        assert_eq!(m.elements.len(), 0);
        assert_eq!(m.elements.capacity(), 1);
    }

    #[test]
    fn test_len_01() {
        let m = ChannelMap::<()>::new(23);
        assert_eq!(m.len(), 0);
    }

    #[test]
    fn test_alloc_01() {
        let mut m = ChannelMap::<()>::new(2);
        assert_eq!(m.alloc(), Some(0));
        assert_eq!(m.len(), 1);
        assert_eq!(m.alloc(), Some(1));
        assert_eq!(m.len(), 2);
        assert_eq!(m.alloc(), None);
    }

    #[test]
    fn test_get_01() {
        let mut m = ChannelMap::<()>::new(2);
        assert_eq!(m.get(0), Err(ConnectionError::ChannelIdInvalid));
    }

    #[test]
    fn test_insert_01() {
        let mut m = ChannelMap::<()>::new(2);
        assert_eq!(m.insert(0, ()), Err(ConnectionError::ChannelIdInvalid));
    }

    #[test]
    fn test_remove_01() {
        let mut m = ChannelMap::<()>::new(2);
        assert_eq!(m.remove(0), Err(ConnectionError::ChannelIdInvalid));
    }

    #[test]
    fn test_alloc_remove_01() {
        let mut m = ChannelMap::<()>::new(2);
        assert_eq!(m.alloc(), Some(0));
        assert_eq!(m.remove(0), Ok(()));
        assert_eq!(m.remove(0), Err(ConnectionError::ChannelIdInvalid));
    }

    #[test]
    fn test_alloc_insert_get_01() {
        let mut m = ChannelMap::<()>::new(2);
        assert_eq!(m.alloc(), Some(0));
        assert_eq!(m.insert(0, ()), Ok(()));
        assert_eq!(m.get(0), Ok(&mut ()));
    }

    #[test]
    fn test_alloc_insert_get_02() {
        let mut m = ChannelMap::<usize>::new(2);
        assert_eq!(m.alloc(), Some(0));
        assert_eq!(m.insert(0, 23), Ok(()));
        assert_eq!(m.get(0), Ok(&mut 23));
        assert_eq!(m.alloc(), Some(1));
        assert_eq!(m.insert(1, 47), Ok(()));
        assert_eq!(m.get(1), Ok(&mut 47));
    }

    #[test]
    fn test_alloc_insert_get_remove_01() {
        let mut m = ChannelMap::<usize>::new(2);
        assert_eq!(m.alloc(), Some(0));
        assert_eq!(m.insert(0, 23), Ok(()));
        assert_eq!(m.get(0), Ok(&mut 23));
        assert_eq!(m.alloc(), Some(1));
        assert_eq!(m.insert(1, 47), Ok(()));
        assert_eq!(m.get(1), Ok(&mut 47));
        assert_eq!(m.remove(0), Ok(()));
        assert_eq!(m.remove(0), Err(ConnectionError::ChannelIdInvalid));
        assert_eq!(m.get(0), Err(ConnectionError::ChannelIdInvalid));
        assert_eq!(m.get(1), Ok(&mut 47));
        assert_eq!(m.alloc(), Some(0));
    }

    #[test]
    fn test_iter_01() {
        let mut m = ChannelMap::<usize>::new(2);
        assert_eq!(m.alloc(), Some(0));
        assert_eq!(m.alloc(), Some(1));
        assert_eq!(m.insert(1, 47), Ok(()));
        assert_eq!(m.get(1), Ok(&mut 47));
        let mut n: usize = 0;
        for i in m.iter() {
            n += 1;
            assert_eq!(i, &mut 47);
        }
        assert_eq!(n, 1);
    }
}
