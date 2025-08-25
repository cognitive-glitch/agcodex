# Basic Usage Guide

This guide walks you through the fundamental features of AGCodex with practical, step-by-step examples.

## 📖 Table of Contents
1. [Starting AGCodex](#starting-agcodex)
2. [Understanding Modes](#understanding-modes)
3. [Basic Code Generation](#basic-code-generation)
4. [Search and Navigate](#search-and-navigate)
5. [Edit and Refactor](#edit-and-refactor)
6. [Session Management](#session-management)
7. [Tips and Best Practices](#tips-and-best-practices)

## Starting AGCodex

### First Launch
```bash
# Basic start
agcodex

# Start in specific mode
agcodex --mode plan    # Read-only planning
agcodex --mode build   # Full access (default)
agcodex --mode review  # Quality focus, limited edits

# Start with specific project
agcodex --project /path/to/project
```

### Initial Setup Checklist
When you first launch AGCodex:

1. **Welcome Screen** - Press Enter to continue
2. **Authentication** - Choose your provider:
   - `1` for OpenAI API key
   - `2` for ChatGPT login
   - `3` for Ollama (local)
3. **Trust Directory** - Approve your working directory
4. **Mode Selection** - AGCodex starts in Build mode by default

**Expected Output:**
```
🚀 AGCodex v2.0.0
📋 Mode: BUILD (Full Access)
🔗 Provider: OpenAI (gpt-4)
📁 Workspace: /home/user/project
─────────────────────────────
Type your message or / for commands...
>
```

## Understanding Modes

AGCodex operates in three distinct modes, switchable with **Shift+Tab**:

### 📋 PLAN Mode (Read-Only)
**Purpose:** Analysis and planning without making changes
```bash
# Example usage in PLAN mode
> Analyze the architecture of this codebase
> What design patterns are used in src/controllers?
> Create a refactoring plan for the database module
```

**Restrictions:**
- ✅ Read files
- ✅ Search codebase
- ✅ Generate plans
- ❌ Write files
- ❌ Execute commands

### 🔨 BUILD Mode (Full Access)
**Purpose:** Active development with all tools available
```bash
# Example usage in BUILD mode
> Create a new REST API endpoint for user profiles
> Refactor the authentication module to use JWT
> Add comprehensive error handling to all controllers
```

**Capabilities:**
- ✅ All PLAN features
- ✅ Create/edit files
- ✅ Execute commands
- ✅ Run tests
- ✅ Git operations

### 🔍 REVIEW Mode (Quality Focus)
**Purpose:** Code review and quality analysis with limited edits
```bash
# Example usage in REVIEW mode
> Review src/auth.rs for security vulnerabilities
> Check test coverage for the API module
> Analyze performance bottlenecks in the data processor
```

**Capabilities:**
- ✅ All PLAN features
- ✅ Small edits (<10KB)
- ✅ Add comments
- ✅ Fix typos
- ❌ Major refactoring

### Mode Switching Example
```
Current mode: PLAN
> I need to fix a bug in auth.rs
< I can see the issue, but I'm in PLAN mode. Press Shift+Tab to switch to BUILD mode.

[Press Shift+Tab]

Current mode: BUILD
> Now fix the bug in auth.rs
< Applying the fix... [shows edit]
```

## Basic Code Generation

### Example 1: Create a Simple Function
```bash
> Create a Python function to validate email addresses

< Here's a Python function for email validation:

```python
import re
from typing import bool

def validate_email(email: str) -> bool:
    """
    Validate an email address using regex pattern.
    
    Args:
        email: The email address to validate
        
    Returns:
        True if valid, False otherwise
    """
    pattern = r'^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$'
    
    if not email or not isinstance(email, str):
        return False
    
    return bool(re.match(pattern, email.strip()))

# Example usage
if __name__ == "__main__":
    test_emails = [
        "user@example.com",      # Valid
        "name.last@company.org",  # Valid
        "invalid.email",          # Invalid
        "@example.com",           # Invalid
        "user@",                  # Invalid
    ]
    
    for email in test_emails:
        result = validate_email(email)
        print(f"{email}: {'Valid' if result else 'Invalid'}")
```

Would you like me to:
1. Save this to a file?
2. Add more validation rules?
3. Create unit tests?
```

### Example 2: Generate a REST API Endpoint
```bash
> Create a REST API endpoint for user registration using FastAPI

< I'll create a user registration endpoint with FastAPI:

```python
from fastapi import FastAPI, HTTPException, Depends
from pydantic import BaseModel, EmailStr, validator
from typing import Optional
from datetime import datetime
import bcrypt
import jwt
from sqlalchemy.orm import Session

app = FastAPI()

# Pydantic models
class UserRegistration(BaseModel):
    email: EmailStr
    username: str
    password: str
    full_name: Optional[str] = None
    
    @validator('username')
    def validate_username(cls, v):
        if len(v) < 3:
            raise ValueError('Username must be at least 3 characters')
        if not v.isalnum():
            raise ValueError('Username must be alphanumeric')
        return v
    
    @validator('password')
    def validate_password(cls, v):
        if len(v) < 8:
            raise ValueError('Password must be at least 8 characters')
        return v

class UserResponse(BaseModel):
    id: int
    email: str
    username: str
    full_name: Optional[str]
    created_at: datetime
    
# Endpoint
@app.post("/api/register", response_model=UserResponse, status_code=201)
async def register_user(user_data: UserRegistration, db: Session = Depends(get_db)):
    """
    Register a new user account.
    
    Returns:
        UserResponse: The created user information
        
    Raises:
        HTTPException: If email or username already exists
    """
    # Check if user exists
    if check_email_exists(db, user_data.email):
        raise HTTPException(status_code=400, detail="Email already registered")
    
    if check_username_exists(db, user_data.username):
        raise HTTPException(status_code=400, detail="Username already taken")
    
    # Hash password
    hashed_password = bcrypt.hashpw(
        user_data.password.encode('utf-8'), 
        bcrypt.gensalt()
    )
    
    # Create user in database
    new_user = User(
        email=user_data.email,
        username=user_data.username,
        password_hash=hashed_password.decode('utf-8'),
        full_name=user_data.full_name,
        created_at=datetime.utcnow()
    )
    
    db.add(new_user)
    db.commit()
    db.refresh(new_user)
    
    return UserResponse(
        id=new_user.id,
        email=new_user.email,
        username=new_user.username,
        full_name=new_user.full_name,
        created_at=new_user.created_at
    )
```

Created the registration endpoint with:
✓ Input validation
✓ Password hashing
✓ Duplicate checking
✓ Error handling
```

## Search and Navigate

### Using the Search Tool
```bash
# Basic search
> search for "TODO" in the codebase

# Search with file pattern
> search for "error" in *.py files

# Search with context
> search for "authenticate" with 3 lines of context

# AST-based search (structure-aware)
> search for all class definitions that inherit from BaseModel

# Complex search
> search for functions with complexity > 10 in src/
```

### Search Examples with Results
```bash
> search for "database connection" in src/

< Found 4 matches across 3 files:

📁 src/db/connection.py
  Line 15-17:
  ```python
  def get_database_connection(config: DBConfig) -> Connection:
      """Establish database connection with retry logic."""
      return psycopg2.connect(**config.dict())
  ```

📁 src/db/pool.py
  Line 42:
  ```python
  # Initialize database connection pool
  ```

📁 src/api/health.py
  Line 8-10:
  ```python
  async def check_database():
      """Health check for database connection."""
      return await db.ping()
  ```

Would you like to:
1. View any of these files in full?
2. Edit a specific occurrence?
3. Refine the search?
```

### File Navigation
```bash
# List files in current directory
> ls

# Find files by pattern
> find all test files

# Browse project structure
> show me the project structure

# Jump to specific file
> open src/main.py
```

## Edit and Refactor

### Simple Edit Example
```bash
> Fix the typo in the comment on line 42 of utils.py

< I'll fix the typo in utils.py:

[Showing edit]
Line 42 before:
  # Calcualte the average of the list

Line 42 after:
  # Calculate the average of the list

✓ Applied edit to utils.py
```

### Refactoring Example
```bash
> Refactor the get_user function to use async/await

< I'll refactor the get_user function to be asynchronous:

[Showing transformation]
Before:
```python
def get_user(user_id: int) -> User:
    conn = get_connection()
    cursor = conn.cursor()
    cursor.execute("SELECT * FROM users WHERE id = ?", (user_id,))
    result = cursor.fetchone()
    conn.close()
    return User(**result) if result else None
```

After:
```python
async def get_user(user_id: int) -> Optional[User]:
    async with get_async_connection() as conn:
        async with conn.cursor() as cursor:
            await cursor.execute("SELECT * FROM users WHERE id = ?", (user_id,))
            result = await cursor.fetchone()
            return User(**result) if result else None
```

Changes applied:
✓ Added async keyword
✓ Converted to async context managers
✓ Added await for async operations
✓ Improved resource management
✓ Added Optional type hint
```

### Bulk Edits
```bash
> Replace all print statements with logger.debug calls

< I'll replace all print statements with proper logging:

Found 12 print statements across 4 files.

[Preview of changes]
📁 src/main.py (3 changes)
  - print(f"Starting application...")
  + logger.debug(f"Starting application...")
  
📁 src/utils.py (5 changes)
  - print(f"Processing {item}")
  + logger.debug(f"Processing {item}")
  
[... more changes ...]

Apply all changes? (y/n): y

✓ Replaced 12 print statements with logger.debug calls
✓ Added import for logger where needed
```

## Session Management

### Saving Sessions
```bash
# Manual save
[Press Ctrl+S]

Save session dialog:
┌─────────────────────────┐
│ Save Session            │
├─────────────────────────┤
│ Name: [refactoring-auth]│
│ Tags: [auth, security]  │
│ Description: [...]      │
└─────────────────────────┘

# Quick save with checkpoint
[Press F5]
✓ Checkpoint created: checkpoint-2024-01-15-14:30
```

### Loading Sessions
```bash
# Browse and load
[Press Ctrl+S, then Tab to switch to Load]

Load session dialog:
┌─────────────────────────────────┐
│ Recent Sessions                 │
├─────────────────────────────────┤
│ 1. feature-api-endpoints (2h)   │
│ 2. bug-fix-auth (yesterday)     │
│ 3. refactoring-database (3d)    │
│ 4. performance-optimization (1w)│
└─────────────────────────────────┘

[Select with arrow keys, Enter to load]
```

### Session Features
```bash
# Auto-save is enabled by default
# Check auto-save status
> show session info

< Current Session:
  ID: 7f3a4b2c-9d8e-4f6a
  Started: 2 hours ago
  Messages: 47
  Mode changes: 3
  Auto-save: Enabled (every 5 minutes)
  Last save: 2 minutes ago
  Size: 124 KB

# Jump to previous message
[Press Ctrl+J]
Jump to message number: 15
✓ Jumped to message 15 with full context restored

# Undo last action
[Press Ctrl+Z]
✓ Undone: "Edit to src/auth.py"

# Create branch from current point
[Press Ctrl+B]
✓ Created branch: session-branch-1
```

## Tips and Best Practices

### 1. Effective Prompting
```bash
# Be specific about requirements
❌ "Create a function"
✅ "Create a Python function to validate ISO date strings with timezone support"

# Provide context
❌ "Fix the bug"
✅ "Fix the null pointer exception in the user authentication flow when email is missing"

# Specify constraints
❌ "Optimize this code"
✅ "Optimize this function to run in O(n log n) time with minimal memory allocation"
```

### 2. Mode Selection Strategy
- **Start in PLAN** when you need to understand before changing
- **Use BUILD** for active development and feature implementation  
- **Switch to REVIEW** for quality checks and code reviews

### 3. Search Optimization
```bash
# Use file patterns for faster searches
> search "class.*Controller" in **/*.py

# Leverage AST search for structure
> find all functions with more than 50 lines

# Combine searches for precision
> search "TODO" in tests/ excluding node_modules/
```

### 4. Session Management Best Practices
- Create checkpoints before major changes (F5)
- Use descriptive names for saved sessions
- Tag sessions for easy discovery
- Branch sessions for experimental changes

### 5. Performance Tips
```bash
# Limit search scope
> search "error" in src/api/  # Faster than searching entire codebase

# Use incremental operations
> refactor step by step with validation

# Enable caching for repeated operations
# In config: cache_enabled = true
```

### 6. Common Workflows

**Bug Fix Workflow:**
```bash
1. Start in PLAN mode
2. Search for error patterns
3. Analyze the issue
4. Switch to BUILD mode (Shift+Tab)
5. Apply fix
6. Run tests
7. Create checkpoint
```

**Feature Development Workflow:**
```bash
1. Start in BUILD mode
2. Create feature branch (git integration)
3. Generate boilerplate
4. Implement logic
5. Add tests
6. Review with REVIEW mode
7. Save session
```

**Code Review Workflow:**
```bash
1. Start in REVIEW mode
2. Analyze code quality
3. Check for vulnerabilities
4. Suggest improvements
5. Add review comments
6. Generate report
```

## Troubleshooting

### Issue: "Cannot edit in current mode"
**Solution:** Press Shift+Tab to switch to BUILD mode

### Issue: "Search taking too long"
**Solution:** Narrow search scope with path filters:
```bash
> search "pattern" in src/controllers/
```

### Issue: "Session not saving"
**Solution:** Check write permissions and disk space:
```bash
> show session info
# Check auto-save status and last save time
```

### Issue: "Agent not responding"
**Solution:** Check mode and agent availability:
```bash
> list available agents
> show current mode
```

## Next Steps

Now that you understand the basics:
1. Explore [Agent Workflows](agent_workflows.md) for multi-agent capabilities
2. Learn [Advanced Features](advanced_features.md) for power user techniques
3. Customize your [Configuration](configuration_templates/) for optimal performance
4. Create [Custom Agents](custom_agents/) for specialized tasks

---

**Quick Reference Card:**
- **Shift+Tab**: Switch modes
- **/**: Command palette
- **Ctrl+S**: Save/load session
- **Ctrl+Z/Y**: Undo/redo
- **F5**: Create checkpoint
- **Esc**: Cancel operation