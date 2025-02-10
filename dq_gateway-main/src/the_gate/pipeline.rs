//#############################################################################################
//#                                 IMPORTANT INFORMATION                                     #
//#############################################################################################
//#   The codebase is at the moment synchronus. This should be amended when we have a working #
//#   prototype. at the moment, if i am not being too doom and gloom,                         #
//#   somewhere around 70%+ of the time used by this approach would likely                    #
//#   be on just waiting for things.                                                          #
//#############################################################################################

use super::*;
//   You can think of the ProcessingPipeline like a smart post office sorting system
// - incoming_queue: Letters that just arrived and need to be sorted
// - outgoing_queue: Letters that have been sorted and are ready to be delivered
// - batch_size: How many letters we process at once (for efficiency)
pub struct ProcessingPipeline {
    incoming_queue: VecDeque<AVLPacket>,
    outgoing_queue: VecDeque<AVLPacket>,
    batch_size: usize,
}

impl ProcessingPipeline {
    // Create a new post office with a specific batch size
    pub fn new(batch_size: usize) -> Self {
        Self {
            incoming_queue: VecDeque::new(),
            outgoing_queue: VecDeque::new(),
            batch_size,
        }
    }

    // Check how many letters we have waiting to be processed and delivered
    // Returns: (letters to process, letters ready for delivery)
    pub fn queue_stats(&self) -> (usize, usize) {
        (self.incoming_queue.len(), self.outgoing_queue.len())
    }

    // Emergency protocol, process all remaining letters right now
    // We will be staying late at the post office to clear the backlog.
    pub fn flush(&mut self) -> io::Result<Vec<AVLPacket>> {
        let mut flushed = Vec::new();
        flushed.extend(self.incoming_queue.drain(..));
        flushed.extend(self.outgoing_queue.drain(..));
        Ok(flushed)
    }

    //   Handle a new incoming packet (letter)
    //   The priority works similar to postal service priority levels:
    // - 0-3: Standard mail (goes to back of queue)
    // - 4-7: Priority mail (gets inserted based on priority)
    // - 8+: Express mail (goes to front of queue)
    pub fn process_incoming(
        &mut self,
        packet: AVLPacket,
        timeout: Option<Duration>,
    ) -> io::Result<()> {
        let start = std::time::Instant::now();

        // Checks the priority level, is this letter sent express?
        let priority = match packet.avl_data.first() {
            Some(data) => data.priority,
            None => 0,
        };

        // Place the packet in the right spot based on how important it is.
        match priority {
            0..=3 => self.incoming_queue.push_back(packet),
            4..=7 => {
                let insert_pos = self
                    .incoming_queue
                    .iter()
                    .position(|p| p.avl_data.first().map_or(0, |d| d.priority) < priority)
                    .unwrap_or(self.incoming_queue.len());
                self.incoming_queue.insert(insert_pos, packet);
            }
            8..=u8::MAX => self.incoming_queue.push_front(packet),
        }

        // Process a batch if:
        // 1. We have enough letters to make a full batch, or
        // 2. We've been holding letters too long
        if self.incoming_queue.len() >= self.batch_size
            || timeout.map_or(false, |t| start.elapsed() >= t)
        {
            self.process_batch()?;
        }

        Ok(())
    }

    // Process a batch of Packets, So, sorting a big bundle of letters at once
    // it is unlikely that you would send out a mailman to deliver just one letter
    fn process_batch(&mut self) -> io::Result<()> {
        let batch: Vec<_> = self
            .incoming_queue
            .drain(..self.batch_size.min(self.incoming_queue.len()))
            .collect();

        for packet in batch {
            self.outgoing_queue.push_back(packet);
        }

        Ok(())
    }
}
