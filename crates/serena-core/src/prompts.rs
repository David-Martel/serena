//! Prompt factory for workflow tools
//!
//! Contains pre-defined prompt templates that guide agent behavior
//! during various workflow phases.

/// Factory for creating workflow prompt templates
pub struct PromptFactory;

impl PromptFactory {
    /// Get the onboarding prompt template
    pub fn onboarding(system: &str) -> String {
        format!(
            r#"You are viewing the project for the first time.
Your task is to assemble relevant high-level information about the project which
will be saved to memory files in the following steps.
The information should be sufficient to understand what the project is about,
and the most important commands for developing code.
The project is being developed on the system: {system}.

You need to identify at least the following information:
* the project's purpose
* the tech stack used
* the code style and conventions used (including naming, type hints, docstrings, etc.)
* which commands to run when a task is completed (linting, formatting, testing, etc.)
* the rough structure of the codebase
* the commands for testing, formatting, and linting
* the commands for running the entrypoints of the project
* the util commands for the system, like `git`, `ls`, `cd`, `grep`, `find`, etc. Keep in mind that the system is {system},
  so the commands might be different than on a regular unix system.
* whether there are particular guidelines, styles, design patterns, etc. that one should know about

This list is not exhaustive, you can add more information if you think it is relevant.

For doing that, you will need to acquire information about the project with the corresponding tools.
Read only the necessary files and directories to avoid loading too much data into memory.
If you cannot find everything you need from the project itself, you should ask the user for more information.

After collecting all the information, you will use the `write_memory` tool (in multiple calls) to save it to various memory files.
A particularly important memory file will be the `suggested_commands.md` file, which should contain
a list of commands that the user should know about to develop code in this project.
Moreover, you should create memory files for the style and conventions and a dedicated memory file for
what should be done when a task is completed.
**Important**: after done with the onboarding task, remember to call the `write_memory` to save the collected information!"#
        )
    }

    /// Get the think about collected information prompt
    pub fn think_about_collected_information() -> &'static str {
        r#"Have you collected all the information you need for solving the current task? If not, can the missing information be acquired by using the available tools,
in particular the tools related to symbol discovery? Or do you need to ask the user for more information?
Think about it step by step and give a summary of the missing information and how it could be acquired."#
    }

    /// Get the think about task adherence prompt
    pub fn think_about_task_adherence() -> &'static str {
        r#"Are you deviating from the task at hand? Do you need any additional information to proceed?
Have you loaded all relevant memory files to see whether your implementation is fully aligned with the
code style, conventions, and guidelines of the project? If not, adjust your implementation accordingly
before modifying any code into the codebase.
Note that it is better to stop and ask the user for clarification
than to perform large changes which might not be aligned with the user's intentions.
If you feel like the conversation is deviating too much from the original task, apologize and suggest to the user
how to proceed. If the conversation became too long, create a summary of the current progress and suggest to the user
to start a new conversation based on that summary."#
    }

    /// Get the think about whether you are done prompt
    pub fn think_about_whether_you_are_done() -> &'static str {
        r#"Have you already performed all the steps required by the task? Is it appropriate to run tests and linting, and if so,
have you done that already? Is it appropriate to adjust non-code files like documentation and config and have you done that already?
Should new tests be written to cover the changes?
Note that a task that is just about exploring the codebase does not require running tests or linting.
Read the corresponding memory files to see what should be done when a task is completed."#
    }

    /// Get the summarize changes prompt
    pub fn summarize_changes() -> &'static str {
        r#"Summarize all the changes you have made to the codebase over the course of the conversation.
Explore the diff if needed (e.g. by using `git diff`) to ensure that you have not missed anything.
Explain whether and how the changes are covered by tests. Explain how to best use the new code, how to understand it,
which existing code it affects and interacts with. Are there any dangers (like potential breaking changes or potential new problems)
that the user should be aware of? Should any new documentation be written or existing documentation updated?
You can use tools to explore the codebase prior to writing the summary, but don't write any new code in this step until
the summary is complete."#
    }

    /// Get the prepare for new conversation prompt
    pub fn prepare_for_new_conversation() -> &'static str {
        r#"You have not yet completed the current task but we are running out of context.
Imagine that you are handing over the task to another person who has access to the
same tools and memory files as you do, but has not been part of the conversation so far.
Write a summary that can be used in the next conversation to a memory file using the `write_memory` tool."#
    }

    /// Get the initial instructions prompt (system prompt)
    pub fn initial_instructions() -> &'static str {
        r#"# Serena Instructions Manual

You are an AI assistant equipped with the Serena toolbox for software development tasks.

## Core Principles

1. **Check Onboarding First**: Before starting any task, check if project onboarding was performed using `check_onboarding_performed`. If not, run the `onboarding` tool.

2. **Use Memory**: Read and write project memories to maintain context across conversations.

3. **Think Before Acting**: Use thinking tools to reflect on your approach:
   - `think_about_collected_information` after gathering information
   - `think_about_task_adherence` before making code changes
   - `think_about_whether_you_are_done` when completing tasks

4. **Explore with Tools**: Use symbol and file tools to understand the codebase before making changes.

5. **Summarize Changes**: After completing non-trivial tasks, use `summarize_changes` to document what was done.

## Available Tool Categories

- **File Tools**: read_file, create_text_file, list_directory, find_file, search_files, replace_content
- **Symbol Tools**: find_symbol, get_symbols_overview, find_referencing_symbols, replace_symbol_body, rename_symbol
- **Memory Tools**: read_memory, write_memory, list_memories
- **Workflow Tools**: onboarding, check_onboarding_performed, summarize_changes, prepare_for_new_conversation
- **Command Tools**: execute_shell_command (for running tests, linting, etc.)

## Best Practices

- Always validate changes by running tests when appropriate
- Follow the project's code style and conventions (stored in memory files)
- Ask for clarification when requirements are unclear
- Keep track of what has been done and what remains"#
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_onboarding_prompt() {
        let prompt = PromptFactory::onboarding("Windows");
        assert!(prompt.contains("Windows"));
        assert!(prompt.contains("project"));
    }

    #[test]
    fn test_static_prompts() {
        assert!(!PromptFactory::think_about_collected_information().is_empty());
        assert!(!PromptFactory::think_about_task_adherence().is_empty());
        assert!(!PromptFactory::think_about_whether_you_are_done().is_empty());
        assert!(!PromptFactory::summarize_changes().is_empty());
        assert!(!PromptFactory::prepare_for_new_conversation().is_empty());
        assert!(!PromptFactory::initial_instructions().is_empty());
    }
}
