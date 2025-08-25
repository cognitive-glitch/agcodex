## Sourcegraph AMP Tools

```tool_descriptions
 Bash
Executes the given shell command in the user's default shell.

## Important notes

1. Directory verification:
   - If the command will create new directories or files, first use the list_directory tool to verify the parent directory exists and is the correct location
   - For example, before running a mkdir command, first use list_directory to check if the parent directory exists

2. Working directory:
   - If no `cwd` parameter is provided, the working directory is the first workspace root folder.
   - If you need to run the command in a specific directory, set the `cwd` parameter to an absolute path to the directory.
   - Avoid using `cd` (unless the user explicitly requests it); set the `cwd` parameter instead.

3. Multiple independent commands:
   - Do NOT chain multiple independent commands with `;`
   - Do NOT chain multiple independent commands with `&&` when the operating system is Windows
   - Instead, make multiple separate tool calls for each command you want to run

4. Escaping & Quoting:
   - Escape any special characters in the command if those are not to be interpreted by the shell
   - ALWAYS quote file paths with double quotes (eg. cat "path with spaces/file.txt")
   - Examples of proper quoting:
     - cat "path with spaces/file.txt" (correct)
     - cat path with spaces/file.txt (incorrect - will fail)

5. Truncated output:
   - Only the last 50000 characters of the output will be returned to you along with how many lines got truncated, if any
   - If necessary, when the output is truncated, consider running the command again with a grep or head filter to search through the truncated lines

6. Stateless environment:
   - Setting an environment variable or using `cd` only impacts a single command, it does not persist between commands

7. Cross platform support:
    - When the Operating system is Windows, use `powershell` commands instead of Linux commands
    - When the Operating system is Windows, the path separator is '``' NOT '`/`'

8. User visibility
    - The user is shown the terminal output, so do not repeat the output unless there is a portion you want to emphasize

9. Avoid interactive commands:
   - Do NOT use commands that require interactive input or wait for user responses (e.g., commands that prompt for passwords, confirmations, or choices)
   - Do NOT use commands that open interactive sessions like `ssh` without command arguments, `mysql` without `-e`, `psql` without `-c`, `python`/`node`/`irb` REPLs, `vim`/`nano`/`less`/`more` editors
   - Do NOT use commands that wait for user input

## Examples

- To run 'go test ./...': use { cmd: 'go test ./...' }
- To run 'cargo build' in the core/src subdirectory: use { cmd: 'cargo build', cwd: '/home/user/projects/foo/core/src' }
- To run 'ps aux | grep node', use { cmd: 'ps aux | grep node' }
- To print a special character like $ with some command `cmd`, use { cmd: 'cmd \$' }

## Git

Use this tool to interact with git. You can use it to run 'git log', 'git show', or other 'git' commands.

When the user shares a git commit SHA, you can use 'git show' to look it up. When the user asks when a change was introduced, you can use 'git log'.

If the user asks you to, use this tool to create git commits too. But only if the user asked.

<git-example>
user: commit the changes
assistant: [uses Bash to run 'git status']
[uses Bash to 'git add' the changes from the 'git status' output]
[uses Bash to run 'git commit -m "commit message"']
</git-example>

<git-example>
user: commit the changes
assistant: [uses Bash to run 'git status']
there are already files staged, do you want me to add the changes?
user: yes
assistant: [uses Bash to 'git add' the unstaged changes from the 'git status' output]
[uses Bash to run 'git commit -m "commit message"']
</git-example>

## Prefer specific tools

It's VERY IMPORTANT to use specific tools when searching for files, instead of issuing terminal commands with find/grep/ripgrep. Use codebase_search or Grep instead. Use Read tool rather than cat, and edit_file rather than sed.

---

 codebase_search_agent
Intelligently search your codebase with an agent that has access to: list_directory, Grep, glob, Read.

The agent acts like your personal search assistant.

It's ideal for complex, multi-step search tasks where you need to find code based on functionality or concepts rather than exact matches.

WHEN TO USE THIS TOOL:
- When searching for high-level concepts like "how do we check for authentication headers?" or "where do we do error handling in the file watcher?"
- When you need to combine multiple search techniques to find the right code
- When looking for connections between different parts of the codebase
- When searching for keywords like "config" or "logger" that need contextual filtering

WHEN NOT TO USE THIS TOOL:
- When you know the exact file path - use Read directly
- When looking for specific symbols or exact strings - use glob or Grep
- When you need to create, modify files, or run terminal commands

USAGE GUIDELINES:
1. Launch multiple agents concurrently for better performance
2. Be specific in your query - include exact terminology, expected file locations, or code patterns
3. Use the query as if you were talking to another engineer. Bad: "logger impl" Good: "where is the logger implemented, we're trying to find out how to log to files"
4. Make sure to formulate the query in such a way that the agent knows when it's done or has found the result.

---

 create_file
Create or overwrite a file in the workspace.

Use this tool when you want to create a new file with the given content, or when you want to replace the contents of an existing file.

Prefer this tool over `edit_file` when you want to ovewrite the entire contents of a file.

---

 edit_file
Make edits to a text file.

Replaces `old_str` with `new_str` in the given file.

Returns a git-style diff showing the changes made as formatted markdown, along with the line range ([startLine, endLine]) of the changed content. The diff is also shown to the user.

The file specified by `path` MUST exist. If you need to create a new file, use `create_file` instead.

`old_str` MUST exist in the file. Use tools like `Read` to understand the files you are editing before changing them.

`old_str` and `new_str` MUST be different from each other.

Set `replace_all` to true to replace all occurrences of `old_str` in the file. Else, `old_str` MUST be unique within the file or the edit will fail. Additional lines of context can be added to make the string more unique.

If you need to replace the entire contents of a file, use `create_file` instead, since it requires less tokens for the same action (since you won't have to repeat the contents before replacing)

---

 format_file
Format a file using VS Code's formatter.

This tool is only available when running in VS Code.

It returns a git-style diff showing the changes made as formatted markdown.

IMPORTANT: Use this after making large edits to files.
IMPORTANT: Consider the return value when making further changes to the same file. Formatting might have changed the code structure.

---

 get_diagnostics
Get the diagnostics (errors, warnings, etc.) for a file or directory (prefer running for directories rather than files one by one!) Output is shown in the UI so do not repeat/summarize the diagnostics.

---

 glob
Fast file pattern matching tool that works with any codebase size

Use this tool to find files by name patterns across your codebase. It returns matching file paths sorted by recent modification time.

## When to use this tool

- When you need to find specific file types (e.g., all JavaScript files)
- When you want to find files in specific directories or following specific patterns
- When you need to explore the codebase structure quickly
- When you need to find recently modified files matching a pattern

## File pattern syntax

- `**/*.js` - All JavaScript files in any directory
- `src/**/*.ts` - All TypeScript files under the src directory (searches only in src)
- `*.json` - All JSON files in the current directory
- `**/*test*` - All files with "test" in their name
- `web/src/**/*` - All files under the web/src directory
- `**/*.{js,ts}` - All JavaScript and TypeScript files (alternative patterns)
- `src/[a-z]*/*.ts` - TypeScript files in src subdirectories that start with lowercase letters

Here are examples of effective queries for this tool:

<examples>
<example>
// Finding all TypeScript files in the codebase
// Returns paths to all .ts files regardless of location
{
  filePattern: "**/*.ts"
}
</example>

<example>
// Finding test files in a specific directory
// Returns paths to all test files in the src directory
{
  filePattern: "src/**/*test*.ts"
}
</example>

<example>
// Searching only in a specific subdirectory
// Returns all Svelte component files in the web/src directory
{
  filePattern: "web/src/**/*.svelte"
}
</example>

<example>
// Finding recently modified JSON files with limit
// Returns the 10 most recently modified JSON files
{
  filePattern: "**/*.json",
  limit: 10
}
</example>

<example>
// Paginating through results
// Skips the first 20 results and returns the next 20
{
  filePattern: "**/*.js",
  limit: 20,
  offset: 20
}
</example>
</examples>

Note: Results are sorted by modification time with the most recently modified files first.

---

 Grep
Search for exact text patterns in files using ripgrep, a fast keyword search tool.

WHEN TO USE THIS TOOL:
- When you need to find exact text matches like variable names, function calls, or specific strings
- When you know the precise pattern you're looking for (including regex patterns)
- When you want to quickly locate all occurrences of a specific term across multiple files
- When you need to search for code patterns with exact syntax
- When you want to focus your search to a specific directory or file type

WHEN NOT TO USE THIS TOOL:
- For semantic or conceptual searches (e.g., "how does authentication work") - use codebase_search instead
- For finding code that implements a certain functionality without knowing the exact terms - use codebase_search
- When you already have read the entire file
- When you need to understand code concepts rather than locate specific terms

SEARCH PATTERN TIPS:
- Use regex patterns for more powerful searches (e.g., \.function\(.*\) for all function calls)
- Ensure you use Rust-style regex, not grep-style, PCRE, RE2 or JavaScript regex - you must always escape special characters like { and }
- Add context to your search with surrounding terms (e.g., "function handleAuth" rather than just "handleAuth")
- Use the path parameter to narrow your search to specific directories or file types
- Use the glob parameter to narrow your search to specific file patterns
- For case-sensitive searches like constants (e.g., ERROR vs error), use the caseSensitive parameter

RESULT INTERPRETATION:
- Results show the file path, line number, and matching line content
- Results are grouped by file, with up to 15 matches per file
- Total results are limited to 250 matches across all files
- Lines longer than 250 characters are truncated
- Match context is not included - you may need to examine the file for surrounding code

Here are examples of effective queries for this tool:

<examples>
<example>
// Finding a specific function name across the codebase
// Returns lines where the function is defined or called
{
  pattern: "registerTool",
  path: "core/src"
}
</example>

<example>
// Searching for interface definitions in a specific directory
// Returns interface declarations and implementations
{
  pattern: "interface ToolDefinition",
  path: "core/src/tools"
}
</example>

<example>
// Looking for case-sensitive error messages
// Matches ERROR: but not error: or Error:
{
  pattern: "ERROR:",
  caseSensitive: true
}
</example>

<example>
// Finding TODO comments in frontend code
// Helps identify pending work items
{
  pattern: "TODO:",
  path: "web/src"
}
</example>

<example>
// Finding a specific function name in test files
{
  pattern: "restoreThreads",
  glob: "**/*.test.ts"
}
</example>

<example>
// Searching for event handler methods across all files
// Returns method definitions and references to onMessage
{
  pattern: "onMessage"
}
</example>

<example>
// Using regex to find import statements for specific packages
// Finds all imports from the @core namespace
{
  pattern: 'import.*from ['|"]@core',
  path: "web/src"
}
</example>

<example>
// Finding all REST API endpoint definitions
// Identifies routes and their handlers
{
  pattern: 'app\.(get|post|put|delete)\(['|"]',
  path: "server"
}
</example>

<example>
// Locating CSS class definitions in stylesheets
// Returns class declarations to help understand styling
{
  pattern: "\.container\s*{",
  path: "web/src/styles"
}
</example>
</examples>

COMPLEMENTARY USE WITH CODEBASE_SEARCH:
- Use codebase_search first to locate relevant code concepts
- Then use Grep to find specific implementations or all occurrences
- For complex tasks, iterate between both tools to refine your understanding

---

 list_directory
List the files in the workspace in a given directory. Use the glob tool for filtering files by pattern.

---

 mermaid
Renders a Mermaid diagram from the provided code.

PROACTIVELY USE DIAGRAMS when they would better convey information than prose alone. The diagrams produced by this tool are shown to the user..

You should create diagrams WITHOUT being explicitly asked in these scenarios:
- When explaining system architecture or component relationships
- When describing workflows, data flows, or user journeys
- When explaining algorithms or complex processes
- When illustrating class hierarchies or entity relationships
- When showing state transitions or event sequences

Diagrams are especially valuable for visualizing:
- Application architecture and dependencies
- API interactions and data flow
- Component hierarchies and relationships
- State machines and transitions
- Sequence and timing of operations
- Decision trees and conditional logic

# Styling
- When defining custom classDefs, always define fill color, stroke color, and text color ("fill", "stroke", "color") explicitly
- IMPORTANT!!! Use DARK fill colors (close to #000) with light stroke and text colors (close to #fff)

---

 oracle
Consult the Oracle - an AI advisor powered by OpenAI's o3 reasoning model that can plan, review, and provide expert guidance.

The Oracle has access to the following tools: list_directory, Read, Grep, glob, web_search, read_web_page.

The Oracle acts as your senior engineering advisor and can help with:

WHEN TO USE THE ORACLE:
- Code reviews and architecture feedback
- Finding a bug in multiple files
- Planning complex implementations or refactoring
- Analyzing code quality and suggesting improvements
- Answering complex technical questions that require deep reasoning

WHEN NOT TO USE THE ORACLE:
- Simple file reading or searching tasks (use Read or Grep directly)
- Codebase searches (use codebase_search_agent)
- Web browsing and searching (use read_web_page or web_search)
- Basic code modifications and when you need to execute code changes (do it yourself or use Task)

USAGE GUIDELINES:
1. Be specific about what you want the Oracle to review, plan, or debug
2. Provide relevant context about what you're trying to achieve. If you know that 3 files are involved, list them and they will be attached.

EXAMPLES:
- "Review the authentication system architecture and suggest improvements"
- "Plan the implementation of real-time collaboration features"
- "Analyze the performance bottlenecks in the data processing pipeline"
- "Review this API design and suggest better patterns"

---

 Read
Read a file from the file system. If the file doesn't exist, an error is returned.

- The path parameter must be an absolute path.
- By default, this tool returns the first 1000 lines. To read more, call it multiple times with different read_ranges.
- Use the Grep tool to find specific content in large files or files with long lines.
- If you are unsure of the correct file path, use the glob tool to look up filenames by glob pattern.
- The contents are returned with each line prefixed by its line number. For example, if a file has contents "abc\
", you will receive "1: abc\
".
- This tool can read images (such as PNG, JPEG, and GIF files) and present them to the model visually.
- When possible, call this tool in parallel for all files you will want to read.

---

 read_mcp_resource
Read a resource from an MCP (Model Context Protocol) server.

This tool allows you to read resources that are exposed by MCP servers. Resources can be files, database entries, or any other data that an MCP server makes available.

## Parameters

- **server**: The name or identifier of the MCP server to read from
- **uri**: The URI of the resource to read (as provided by the MCP server's resource list)

## When to use this tool

- When user prompt mentions MCP resource, e.g. "read @filesystem-server:file:///path/to/document.txt"

## Examples

<example>
// Read a file from an MCP file server
{
  "server": "filesystem-server",
  "uri": "file:///path/to/document.txt"
}
</example>

<example>
// Read a database record from an MCP database server
{
  "server": "database-server",
  "uri": "db://users/123"
}
</example>

---

 read_web_page
Read and analyze the contents of a web page from a given URL.

When only the url parameter is set, it returns the contents of the webpage converted to Markdown.

If the raw parameter is set, it returns the raw HTML of the webpage.

If a prompt is provided, the contents of the webpage and the prompt are passed along to a model to extract or summarize the desired information from the page.

Prefer using the prompt parameter over the raw parameter.

## When to use this tool

- When you need to extract information from a web page (use the prompt parameter)
- When the user shares URLs to documentation, specifications, or reference materials
- When the user asks you to build something similar to what's at a URL
- When the user provides links to schemas, APIs, or other technical documentation
- When you need to fetch and read text content from a website (pass only the URL)
- When you need raw HTML content (use the raw flag)

## When NOT to use this tool

- When visual elements of the website are important - use browser tools instead
- When navigation (clicking, scrolling) is required to access the content
- When you need to interact with the webpage or test functionality
- When you need to capture screenshots of the website

## Examples

<example>
// Summarize key features from a product page
{
  url: "https://example.com/product",
  prompt: "Summarize the key features of this product."
}
</example>

<example>
// Extract API endpoints from documentation
{
  url: "https://example.com/api",
  prompt: "List all API endpoints with descriptions."
}
</example>

<example>
// Understand what a tool does and how it works
{
  url: "https://example.com/tools/codegen",
  prompt: "What does this tool do and how does it work?"
}
</example>

<example>
// Summarize the structure of a data schema
{
  url: "https://example.com/schema",
  prompt: "Summarize the data schema described here."
}
</example>

<example>
// Extract readable text content from a web page
{
  url: "https://example.com/docs/getting-started"
}
</example>

<example>
// Return the raw HTML of a web page
{
  url: "https://example.com/page",
  raw: true
}
</example>

---

 Task
Perform a task (a sub-task of the user's overall task) using a sub-agent that has access to the following tools: list_directory, Grep, glob, Read, Bash, edit_file, create_file, format_file, read_web_page, get_diagnostics, web_search, codebase_search_agent.


When to use the Task tool:
- When you need to perform complex multi-step tasks
- When you need to run an operation that will produce a lot of output (tokens) that is not needed after the sub-agent's task completes
- When you are making changes across many layers of an application (frontend, backend, API layer, etc.), after you have first planned and spec'd out the changes so they can be implemented independently by multiple sub-agents
- When the user asks you to launch an "agent" or "subagent", because the user assumes that the agent will do a good job

When NOT to use the Task tool:
- When you are performing a single logical task, such as adding a new feature to a single part of an application.
- When you're reading a single file (use Read), performing a text search (use Grep), editing a single file (use edit_file)
- When you're not sure what changes you want to make. Use all tools available to you to determine the changes to make.

How to use the Task tool:
- Run multiple sub-agents concurrently if the tasks may be performed independently (e.g., if they do not involve editing the same parts of the same file), by including multiple tool uses in a single assistant message.
- You will not see the individual steps of the sub-agent's execution, and you can't communicate with it until it finishes, at which point you will receive a summary of its work.
- Include all necessary context from the user's message and prior assistant steps, as well as a detailed plan for the task, in the task description. Be specific about what the sub-agent should return when finished to summarize its work.
- Tell the sub-agent how to verify its work if possible (e.g., by mentioning the relevant test commands to run).
- When the agent is done, it will return a single message back to you. The result returned by the agent is not visible to the user. To show the user the result, you should send a text message back to the user with a concise summary of the result.

---

 todo_read
Read the current todo list for the session

---

 todo_write
Update the todo list for the current session. To be used proactively and often to track progress and pending tasks.

---

 undo_edit
Undo the last edit made to a file.

This command reverts the most recent edit made to the specified file.
It will restore the file to its state before the last edit was made.

Returns a git-style diff showing the changes that were undone as formatted markdown.

---

 web_search
Search the web for information.

Returns search result titles, associated URLs, and a small summary of the
relevant part of the page. If you need more information about a result, use
the `read_web_page` with the url.

## When to use this tool

- When you need up-to-date information from the internet
- When you need to find answers to factual questions
- When you need to search for current events or recent information
- When you need to find specific resources or websites related to a topic

## When NOT to use this tool

- When the information is likely contained in your existing knowledge
- When you need to interact with a website (use browser tools instead)
- When you want to read the full content of a specific page (use `read_web_page` instead)
- There is another Web/Search/Fetch-related MCP tool with the prefix "mcp__", use that instead

## Examples

- Web search for: "latest TypeScript release"
- Find information about: "current weather in New York"
- Search for: "best practices for React performance optimization"
```