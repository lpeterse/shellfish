use super::*;
use std::slice::IterMut;

pub struct ChannelMap {
    capacity: usize,
    elements: Vec<Option<ChannelState>>,
}

impl ChannelMap {
    pub fn new(capacity: usize) -> Self {
        ChannelMap {
            capacity,
            elements: Vec::with_capacity(1),
        }
    }

    pub fn len(&self) -> usize {
        let mut len = 0;
        for e in &self.elements {
            if e.is_some() {
                len += 1
            };
        }
        len
    }

    pub fn get(&mut self, id: u32) -> Result<&mut ChannelState, ConnectionError> {
        match self.elements.get_mut(id as usize) {
            Some(Some(t)) => Ok(t),
            _ => Err(ConnectionError::InvalidChannelId),
        }
    }

    pub fn free(&self) -> Option<u32> {
        for (i,e) in self.elements.iter().enumerate() {
            match e {
                None => return Some(i as u32),
                _ => ()
            }
        }
        if self.elements.len() < self.capacity {
            return Some(self.elements.len() as u32);
        }
        None
    }

    pub fn insert(&mut self, t: ChannelState) -> Result<(), ConnectionError> {
        // FIXME
        self.elements.push(Some(t));
        Ok(())
    }

    pub fn remove(&mut self, id: u32) {
        if (id as usize) < self.elements.len() {
            self.elements[id as usize] = None;
        }
    }

    pub fn terminate(&mut self, e: ConnectionError) {
        for element in &mut self.elements {
            match element {
                None => (),
                Some(_) => match std::mem::replace(element, None) {
                    Some(c) => c.terminate(e),
                    None => (),
                },
            }
        }
    }

    pub fn iter<'a>(&'a mut self) -> ChannelMapIterator<'a> {
        ChannelMapIterator(self.elements.iter_mut())
    }
}

pub struct ChannelMapIterator<'a>(IterMut<'a, Option<ChannelState>>);

impl<'a> Iterator for ChannelMapIterator<'a> {
    type Item = &'a mut ChannelState;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.0.next() {
                None => return None,
                Some(None) => continue, // skip
                Some(Some(t)) => return Some(t),
            }
        }
    }
}
