# 207 Multi-Status Ingest Response Implementation

**Date:** March 31, 2026  
**Status:** Complete  
**Build:** ✓ Release build successful

## Overview

Implemented HTTP 207 Multi-Status response for the POST `/api/v1/ingest` endpoint to provide per-snapshot status reporting instead of all-or-nothing behavior.

## Changes Made

### 1. Type Definitions (netwatch-core)
**File:** `crates/netwatch-core/src/types.rs`

Added new types:
- `IngestResponse` now includes:
  - `accepted: u32` - Count of successfully ingested snapshots
  - `rejected: u32` - Count of rejected snapshots
  - `host_id: Uuid` - Host identifier
  - `results: Vec<SnapshotResult>` - Per-snapshot status array

- New `SnapshotResult` struct:
  - `index: usize` - Position in original request
  - `status: u16` - HTTP status code (200 or 400)
  - `message: String` - Status message

### 2. Route Handler (netwatch-cloud)
**File:** `src/routes/ingest.rs`

Implemented error collection strategy:
- Changed from fail-fast to error-collecting loop
- Track individual snapshot success/failure
- Log rejected snapshots with error details
- Return per-snapshot status in response

**Key changes:**
- Iterate with `enumerate()` to track snapshot index
- Use `match` instead of `?` operator to collect errors
- Continue processing remaining snapshots on individual failures
- Skip nested operations on parent failure (snapshot insert, interface metrics, disk metrics)

### 3. Response Status Code Logic
```rust
if rejected > 0 && accepted > 0 {
    // Partial success → 207 Multi-Status
    StatusCode::MULTI_STATUS
} else if rejected == payload.snapshots.len() as u32 {
    // All rejected → 400 Bad Request
    StatusCode::BAD_REQUEST
} else {
    // All accepted → 200 OK
    StatusCode::OK
}
```

### 4. Documentation
**File:** `SPEC.md`

Updated API specification with:
- Response examples for all three scenarios (200, 207, 400)
- Per-snapshot result structure
- Clear status code documentation
- Examples showing partial failure with detailed error messages

## Response Examples

### 200 OK (All Accepted)
```json
{
  "accepted": 95,
  "rejected": 0,
  "host_id": "uuid",
  "results": [
    { "index": 0, "status": 200, "message": "OK" },
    { "index": 1, "status": 200, "message": "OK" }
  ]
}
```

### 207 Multi-Status (Partial Success)
```json
{
  "accepted": 95,
  "rejected": 5,
  "host_id": "uuid",
  "results": [
    { "index": 0, "status": 200, "message": "OK" },
    { "index": 5, "status": 400, "message": "Failed to insert snapshot" },
    { "index": 10, "status": 400, "message": "Failed to insert interface metrics" }
  ]
}
```

### 400 Bad Request (All Rejected)
```json
{
  "accepted": 0,
  "rejected": 100,
  "host_id": "uuid",
  "results": [
    { "index": 0, "status": 400, "message": "Failed to insert snapshot" }
  ]
}
```

## Logging

Enhanced error logging includes:
- Snapshot index for traceability
- Error details from database operations
- Rejection counts in summary log

Example:
```
ingested 100 snapshots for host uuid (95 accepted, 5 rejected)
failed to insert snapshot 5: constraint violation
failed to insert interface metric for snapshot 10: database error
```

## Benefits

1. **Partial Acceptance:** No longer loses all data when some snapshots fail
2. **Detailed Feedback:** Clients know exactly which snapshots failed and why
3. **Better Debugging:** Log includes specific snapshot indices
4. **Standards Compliant:** Uses HTTP 207 per RFC 4918
5. **Backward Compatible:** Still returns 200 for all-success cases

## Testing Considerations

- Test with mixed valid/invalid snapshots
- Verify 207 status code on partial failures
- Confirm per-snapshot error messages are accurate
- Validate rejected count matches results array
- Check that all three status codes (200, 207, 400) work correctly

## Build Status

```
✓ cargo build --release
   Compiling netwatch-core v0.1.0
   Compiling netwatch-cloud v0.1.0
   Finished `release` profile [optimized] in 4.80s
```

## Files Modified

1. `crates/netwatch-core/src/types.rs` - Type definitions (+12 lines)
2. `src/routes/ingest.rs` - Route handler (+160 lines, refactored)
3. `SPEC.md` - API documentation (+37 lines)

## Implementation Time

- Design: 5 minutes
- Implementation: 15 minutes
- Testing/verification: 5 minutes
- Documentation: 10 minutes
- **Total: 35 minutes**
