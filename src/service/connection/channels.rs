use super::*;

use std::slice::IterMut;

#[derive(Debug)]
pub(crate) struct Channels<T = Box<dyn ChannelState>> {
    capacity: usize,
    elements: Vec<Option<T>>,
}

impl<T> Channels<T> {
    pub fn new(capacity: u32) -> Self {
        Channels {
            capacity: capacity as usize,
            elements: Vec::with_capacity(1),
        }
    }

    pub fn get(&mut self, id: u32) -> Result<&mut T, ConnectionError> {
        match self.elements.get_mut(id as usize) {
            Some(Some(t)) => Ok(t),
            _ => Err(ConnectionError::ChannelIdInvalid),
        }
    }

    pub fn free_id(&mut self) -> Option<u32> {
        Some(0) // FIXME
    }

    pub fn insert(&mut self, id: u32, channel: T) -> Result<(), ConnectionError> {
        if let Some(x) = self.elements.get_mut(id as usize) {
            if x.is_none() {
                *x = Some(channel);
                return Ok(());
            }
        }
        Err(ConnectionError::ChannelIdInvalid)
        /*
        for id in 0..self.elements.len() {
            match self.elements[id] {
                None => {
                    self.elements[id] = Some(f(id as u32));
                    return Some(id as u32);
                }
                _ => (),
            }
        }
        if self.elements.len() < self.capacity {
            let id = self.elements.len() as u32;
            self.elements.push(Some(f(id)));
            return Some(id);
        }
        None
        */
    }

    pub fn remove(&mut self, id: u32) -> Result<(), ConnectionError> {
        if let Some(x) = self.elements.get_mut(id as usize) {
            if x.is_some() {
                *x = None;
                return Ok(());
            }
        }
        Err(ConnectionError::ChannelIdInvalid)
    }

    pub fn iter<'a>(&'a mut self) -> ChannelsIterator<'a, T> {
        ChannelsIterator(self.elements.iter_mut())
    }

    pub fn terminate(&mut self, _e: ConnectionError) {
        // FIXME
        todo!()
    }
}

pub struct ChannelsIterator<'a, T>(IterMut<'a, Option<T>>);

impl<'a, T> Iterator for ChannelsIterator<'a, T> {
    type Item = &'a mut T;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.0.next() {
                None => return None,
                Some(None) => continue,
                Some(Some(t)) => return Some(t),
            }
        }
    }
}

/*
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_01() {
        let m = Channels::<()>::new(23);
        assert_eq!(m.capacity, 23);
        assert_eq!(m.elements.len(), 0);
        assert_eq!(m.elements.capacity(), 1);
    }

    #[test]
    fn test_len_01() {
        let m = Channels::<()>::new(23);
        assert_eq!(m.len(), 0);
    }

    #[test]
    fn test_alloc_01() {
        let mut m = Channels::<()>::new(2);
        assert_eq!(m.alloc(), Some(0));
        assert_eq!(m.len(), 1);
        assert_eq!(m.alloc(), Some(1));
        assert_eq!(m.len(), 2);
        assert_eq!(m.alloc(), None);
    }

    #[test]
    fn test_get_01() {
        let mut m = Channels::<()>::new(2);
        assert_eq!(m.get(0), Err(ConnectionError::ChannelIdInvalid));
    }

    #[test]
    fn test_insert_01() {
        let mut m = Channels::<()>::new(2);
        assert_eq!(m.insert(0, ()), Err(ConnectionError::ChannelIdInvalid));
    }

    #[test]
    fn test_remove_01() {
        let mut m = Channels::<()>::new(2);
        assert_eq!(m.remove(0), Err(ConnectionError::ChannelIdInvalid));
    }

    #[test]
    fn test_alloc_remove_01() {
        let mut m = Channels::<()>::new(2);
        assert_eq!(m.alloc(), Some(0));
        assert_eq!(m.remove(0), Ok(()));
        assert_eq!(m.remove(0), Err(ConnectionError::ChannelIdInvalid));
    }

    #[test]
    fn test_alloc_insert_get_01() {
        let mut m = Channels::<()>::new(2);
        assert_eq!(m.alloc(), Some(0));
        assert_eq!(m.insert(0, ()), Ok(()));
        assert_eq!(m.get(0), Ok(&mut ()));
    }

    #[test]
    fn test_alloc_insert_get_02() {
        let mut m = Channels::<usize>::new(2);
        assert_eq!(m.alloc(), Some(0));
        assert_eq!(m.insert(0, 23), Ok(()));
        assert_eq!(m.get(0), Ok(&mut 23));
        assert_eq!(m.alloc(), Some(1));
        assert_eq!(m.insert(1, 47), Ok(()));
        assert_eq!(m.get(1), Ok(&mut 47));
    }

    #[test]
    fn test_alloc_insert_get_remove_01() {
        let mut m = Channels::<usize>::new(2);
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
        let mut m = Channels::<usize>::new(2);
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
*/
