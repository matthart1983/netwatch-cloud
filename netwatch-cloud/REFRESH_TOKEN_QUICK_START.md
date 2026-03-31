# Refresh Token Quick Start

## API Usage Examples

### 1. Register New User

```bash
curl -X POST http://localhost:8080/api/v1/auth/register \
  -H "Content-Type: application/json" \
  -d '{
    "email": "user@example.com",
    "password": "securepass123"
  }'
```

**Response:**
```json
{
  "account_id": "550e8400-e29b-41d4-a716-446655440000",
  "api_key": "nw_ak_...",
  "access_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "refresh_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."
}
```

### 2. Login Existing User

```bash
curl -X POST http://localhost:8080/api/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "email": "user@example.com",
    "password": "securepass123"
  }'
```

**Response:**
```json
{
  "account_id": "550e8400-e29b-41d4-a716-446655440000",
  "access_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "refresh_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."
}
```

### 3. Make Authenticated Request

```bash
curl -X GET http://localhost:8080/api/v1/hosts \
  -H "Authorization: Bearer <access_token>"
```

### 4. Refresh Token (Before Expiry)

```bash
curl -X POST http://localhost:8080/api/v1/auth/refresh \
  -H "Content-Type: application/json" \
  -d '{
    "refresh_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."
  }'
```

**Response:**
```json
{
  "access_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "refresh_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."
}
```

---

## Client Implementation Pattern

```javascript
// JavaScript/TypeScript example
class AuthClient {
  constructor(apiUrl) {
    this.apiUrl = apiUrl;
    this.accessToken = localStorage.getItem('access_token');
    this.refreshToken = localStorage.getItem('refresh_token');
  }

  async login(email, password) {
    const response = await fetch(`${this.apiUrl}/api/v1/auth/login`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ email, password })
    });
    
    const data = await response.json();
    this.accessToken = data.access_token;
    this.refreshToken = data.refresh_token;
    
    localStorage.setItem('access_token', this.accessToken);
    localStorage.setItem('refresh_token', this.refreshToken);
    
    return data;
  }

  async refreshTokens() {
    const response = await fetch(`${this.apiUrl}/api/v1/auth/refresh`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ refresh_token: this.refreshToken })
    });
    
    const data = await response.json();
    this.accessToken = data.access_token;
    this.refreshToken = data.refresh_token;
    
    localStorage.setItem('access_token', this.accessToken);
    localStorage.setItem('refresh_token', this.refreshToken);
    
    return data;
  }

  async apiCall(endpoint, options = {}) {
    const response = await fetch(`${this.apiUrl}${endpoint}`, {
      ...options,
      headers: {
        ...options.headers,
        'Authorization': `Bearer ${this.accessToken}`
      }
    });
    
    // If access token expired, refresh and retry
    if (response.status === 401) {
      await this.refreshTokens();
      return this.apiCall(endpoint, options); // Retry
    }
    
    return response;
  }
}

// Usage
const client = new AuthClient('https://api.example.com');
await client.login('user@example.com', 'password');
const hosts = await client.apiCall('/api/v1/hosts');
```

---

## Token Expiration Timeline

| Event | Time |
|-------|------|
| User logs in | t=0 |
| Access token expires | t=15 min |
| **User should refresh** | t=14 min (proactive) |
| Refresh token expires | t=7 days |
| **User must re-login** | After t=7 days |

---

## Recommended Client Strategy

1. **Store tokens:**
   - Access token: localStorage or sessionStorage
   - Refresh token: localStorage (persists across sessions)

2. **Check token expiry:**
   - Decode JWT and check `exp` claim
   - Before it expires, call refresh endpoint

3. **Handle 401 responses:**
   - Refresh tokens and retry the request
   - If refresh fails, redirect to login

4. **Proactive refresh:**
   - Set timer to refresh at 80% of expiry (12 min for access token)
   - Ensures uninterrupted experience

---

## Token Claims Structure

### Access Token
```json
{
  "sub": "550e8400-e29b-41d4-a716-446655440000",
  "token_type": "access",
  "exp": 1711948200,
  "iat": 1711947300
}
```

### Refresh Token
```json
{
  "sub": "550e8400-e29b-41d4-a716-446655440000",
  "token_type": "refresh",
  "exp": 1712552200,
  "iat": 1711947300
}
```

**Fields:**
- `sub` - Account ID (subject)
- `token_type` - "access" or "refresh"
- `exp` - Unix timestamp of expiration
- `iat` - Unix timestamp of issuance

---

## Error Handling

| Error | HTTP Status | Cause | Action |
|-------|------------|-------|--------|
| Invalid signature | 401 | Token tampered | Re-login |
| Token expired | 401 | Token reached expiry | Refresh or re-login |
| Wrong token type | 401 | Used access as refresh | Refresh tokens |
| Account not found | 500 | Account deleted | Re-login |
| Missing refresh_token | 400 | Bad request | Check payload |

---

## Security Best Practices

✅ Store refresh tokens securely (httpOnly cookies or encrypted storage)  
✅ Always use HTTPS for token transmission  
✅ Never log tokens in debug output  
✅ Implement token refresh before expiry (not after)  
✅ Clear tokens on logout  
✅ Validate token_type in client-side checks  

---

**Learn more:** See [REFRESH_TOKEN_IMPLEMENTATION.md](./REFRESH_TOKEN_IMPLEMENTATION.md)
