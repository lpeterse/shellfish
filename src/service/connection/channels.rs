use super::*;

use std::slice::IterMut;

#[derive(Debug)]
pub(crate) struct Channels<T = ChannelState> {
    capacity: usize,
    elements: Vec<Option<T>>,
}

impl<T> Channels<T> {
    pub fn new(capacity: usize) -> Self {
        Channels {
            capacity,
            elements: Vec::with_capacity(1),
        }
    }

    pub fn get(&mut self, id: u32) -> Result<&mut T, ConnectionError> {
        match self.elements.get_mut(id as usize) {
            Some(Some(t)) => Ok(t),
            _ => Err(ConnectionError::ChannelIdInvalid),
        }
    }

    pub fn get_free_id(&mut self) -> Option<u32> {
        for (i, c) in self.elements.iter().enumerate() {
            if c.is_none() {
                return Some(i as u32);
            }
        }
        if self.elements.len() < self.capacity {
            self.elements.push(None);
            Some(self.elements.len() as u32 - 1)
        } else {
            None
        }
    }

    pub fn insert(&mut self, id: u32, channel: T) -> Result<(), ConnectionError> {
        if let Some(x) = self.elements.get_mut(id as usize) {
            if x.is_none() {
                *x = Some(channel);
                return Ok(());
            }
        }
        Err(ConnectionError::ChannelIdInvalid)
    }

    pub fn remove(&mut self, id: u32) -> Result<T, ConnectionError> {
        if let Some(x) = self.elements.get_mut(id as usize) {
            if let Some(ch) = x.take() {
                return Ok(ch);
            }
        }
        Err(ConnectionError::ChannelIdInvalid)
    }

    pub fn iter_mut<'a>(&'a mut self) -> IterMut<'a, Option<T>> {
        self.elements.iter_mut()
    }
}

impl Terminate for Channels {
    fn terminate(&mut self, e: ConnectionError) {
        for slot in self.iter_mut() {
            if let Some(channel) = slot {
                channel.terminate(e)
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
