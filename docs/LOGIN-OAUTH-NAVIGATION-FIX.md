# Login OAuth Fix - Navigation Handler

## Problem

When users tried to log in, they saw the error:
```
"Oops! Something unexpected happened.
Try refreshing and if the issue persists please contact support.
Failure code: 1779386796589"
```

## Root Cause

The OAuth login flow involves multiple cross-domain redirects:
1. User clicks login on `chat.qwen.ai`
2. Qwen's app calls `window.open()` to open auth page
3. Our bridge intercepts and navigates to `accounts.qwen.ai` or `passport.alibaba.com`
4. After auth, provider redirects back to `chat.qwen.ai/callback` with tokens

**The issue**: WebKitGTK wasn't properly handling these cross-domain navigations. Without explicit navigation handling, some redirects were being blocked or not properly maintaining session state across domains.

## Solution

Added `on_navigation` handler to both main window and new windows that:

1. **Allows all auth-related domain navigations** - Ensures OAuth flow stays within webview
2. **Allows all auth-related path navigations** - Handles `/login`, `/auth`, `/callback`, etc.
3. **Allows navigation back to chat.qwen.ai** - Ensures OAuth callback completes
4. **Allows other HTTPS/HTTP navigation** - For normal browsing (handled by `open_external_link` for external links)

## Changes Made

### 1. Navigation Handler (`src/lib.rs` and `src/window.rs`)

Added `.on_navigation()` handler to both main window and new windows:

```rust
.on_navigation(|url| {
    let url_str = url.to_string();
    let auth_domains = [
        "chat.qwen.ai",
        "accounts.qwen.ai",
        "account.qwen.ai",
        "login.qwen.ai",
        "auth.qwen.ai",
        "oauth.qwen.ai",
        "passport.alibaba.com",
        "login.alibaba.com",
        "signin.alibaba.com",
        "accounts.alibaba.com",
        "account.alibaba.com",
        "login.aliyun.com",
        "account.aliyun.com",
        "signin.aliyun.com",
    ];
    
    let is_auth_domain = auth_domains.iter().any(|domain| url_str.contains(domain));
    let is_auth_path = url_str.contains("/login") 
        || url_str.contains("/auth") 
        || url_str.contains("/oauth")
        || url_str.contains("/callback")
        || url_str.contains("/signin")
        || url_str.contains("/signup");
    
    // Allow navigation if it's auth-related or back to chat.qwen.ai
    if is_auth_domain || is_auth_path || url_str.starts_with("https://chat.qwen.ai") {
        true
    } else {
        url_str.starts_with("https://") || url_str.starts_with("http://")
    }
})
```

### 2. OAuth Callback Handler (`electron-bridge.js`)

Added JavaScript handler to detect and process OAuth callbacks:

```javascript
(function handleOAuthCallback() {
  var currentUrl = window.location.href;
  var isCallback = /callback|oauth.*callback|auth.*callback/.test(currentUrl) ||
    (currentUrl.includes('chat.qwen.ai') && 
     (currentUrl.includes('token=') || currentUrl.includes('code=') || 
      currentUrl.includes('sid=') || currentUrl.includes('ticket=')));

  if (!isCallback) return;

  console.log('[ElectronBridge] OAuth callback detected, extracting tokens...');

  // Parse URL parameters
  var urlParams = new URLSearchParams(window.location.search);
  var token = urlParams.get('token') || urlParams.get('code') || 
              urlParams.get('sid') || urlParams.get('ticket');

  if (!token) {
    console.warn('[ElectronBridge] No token found in callback URL');
    return;
  }

  // Store token in localStorage and cookies
  try {
    localStorage.setItem('token', token);
    localStorage.setItem('sid', token);
    localStorage.setItem('ticket', token);
    localStorage.setItem('auth_token', token);
    localStorage.setItem('qwen_auth_token', token);
    sessionStorage.setItem('token', token);
    sessionStorage.setItem('sid', token);

    document.cookie = 'token=' + token + '; domain=.qwen.ai; path=/; max-age=2592000';
    document.cookie = 'sid=' + token + '; domain=.qwen.ai; path=/; max-age=2592000';
    document.cookie = 'ticket=' + token + '; domain=.qwen.ai; path=/; max-age=2592000';

    console.log('[ElectronBridge] Token stored successfully');

    // Redirect to main chat page
    window.location.href = 'https://chat.qwen.ai';
  } catch(e) {
    console.error('[ElectronBridge] Failed to store token:', e);
  }
})();
```

## Why This Was Needed

The typical OAuth flow in web apps uses a popup window:
1. Main app opens popup to auth provider
2. User authenticates in popup
3. Auth provider redirects popup to callback URL
4. Callback page uses `window.opener.postMessage()` to send tokens back
5. Popup closes itself

However, our bridge intercepts `window.open()` and navigates the current window instead. This breaks the `window.opener` reference, so the callback page can't communicate back to the main app.

**The solution**: Detect callback URLs and manually extract/store tokens, then redirect to the main page.

## How OAuth Flow Works Now

1. **User clicks login** → `chat.qwen.ai` calls `window.open('https://accounts.qwen.ai/...')`
2. **Bridge intercepts** → Navigates current window to auth URL (stays in webview)
3. **User authenticates** → On `accounts.qwen.ai` or `passport.alibaba.com`
4. **OAuth callback** → Provider redirects to `chat.qwen.ai/callback?token=...`
5. **Navigation handler allows** → Redirect completes within webview
6. **Session established** → User is logged in ✅

## Testing

1. Click login button
2. Authenticate with Qwen account
3. Should redirect back to chat.qwen.ai and be logged in
4. No "Oops!" error should appear

## Related Files

- `src/lib.rs` - Main window navigation handler
- `src/window.rs` - New window navigation handler
- `electron-bridge.js` - `window.open()` interception (already in place)
- `src/window.rs::open_external_link()` - External link handling (already in place)

## Notes

- The `window.open()` interception in `electron-bridge.js` still handles the initial auth URL
- The `on_navigation` handler ensures all subsequent redirects in the OAuth flow are allowed
- Cross-domain cookies are automatically shared within the same webview session
- This fix applies to both main window and any additional windows (Ctrl+N)
