# Status Message Auto-Clear Design

## Overview

Fix the bug where status messages persist indefinitely. Add a timestamp when messages are set and auto-clear them after 3 seconds in the main loop.

## Changes

### `crates/gitat-ui/src/app.rs`

Add `status_message_set_at: Option<Instant>` field to `App`. Update `set_status_message` to record `Instant::now()`. Add `clear_expired_status_message` method that clears the message if 3 seconds have elapsed.

```rust
use std::time::{Duration, Instant};

// New field in App:
pub status_message_set_at: Option<Instant>,

// Updated method:
pub fn set_status_message(&mut self, msg: impl Into<String>) {
    self.status_message = Some(msg.into());
    self.status_message_set_at = Some(Instant::now());
}

// New method:
pub fn clear_expired_status_message(&mut self) {
    if let Some(set_at) = self.status_message_set_at {
        if set_at.elapsed() > Duration::from_secs(3) {
            self.status_message = None;
            self.status_message_set_at = None;
        }
    }
}
```

### `crates/gitat/src/main.rs`

Call `app.clear_expired_status_message()` at the start of each loop iteration, before `terminal.draw`.

## Testing

- Unit test: verify `clear_expired_status_message` clears message after expiry (use `status_message_set_at = Some(Instant::now() - Duration::from_secs(4))` to simulate elapsed time)
- Unit test: verify message is NOT cleared before expiry
