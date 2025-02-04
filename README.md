# WebAuthn Rust - Backend for Polling Application

## Overview
`webauthn_rust` is a Rust-based backend application that provides secure authentication using WebAuthn (passkeys) and serves as the backend for a polling system. Built with `actix-web`, this backend handles authentication, session management, and polling operations efficiently.

## Features
- **Passkey Authentication:** Secure WebAuthn-based authentication without passwords.
- **Polling API:** Endpoints for creating, managing, voting on, resetting, and closing polls.
- **WebSockets Support:** Enables real-time interactions.
- **CORS Enabled:** Allows communication with frontend clients.
- **Session Management:** Uses `actix-session` for handling user sessions.

## Technologies Used
- **Rust** (Backend language)
- **Actix-web** (Web framework)
- **WebAuthn-rs** (Passkey authentication)
- **Actix-session** (Session management)
- **Actix-cors** (CORS handling)
- **WebSockets** (For real-time updates)
- **PostgreSQL** (For poll data storage, optional integration)

## API Endpoints
### Authentication
- `POST /register/start/{username}` - Begin user registration.
- `POST /register/finish` - Complete user registration.
- `POST /login/start/{username}` - Start authentication.
- `POST /login/finish` - Complete authentication.

### Poll Management
- `POST /poll/new` - Create a new poll.
- `POST /polls` - Fetch all polls.
- `POST /polls/{poll_id}/vote` - Vote on a poll.
- `GET /polls/{poll_id}` - Get poll details.
- `POST /polls/manage` - Manage user polls.
- `POST /polls/{poll_id}/close` - Close a poll.
- `POST /polls/{poll_id}/reset` - Reset poll votes.

### WebSockets
- `GET /ws` - Establish a WebSocket connection.

## Roadmap & Future Enhancements
- Implement persistent session storage (e.g., Redis).
- Enhance WebSocket support for real-time poll updates.
- Add role-based access control (RBAC) for better user management.
- Improve database integration for scalable polling storage.

