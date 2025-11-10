# Development Workflow

> **Purpose:** Define the step-by-step process for implementing the quote streaming system using `tasklist.md` and `vision.md`.

---

## 🎯 Core Principle

**Work strictly according to `tasklist.md` plan. No deviations without approval.**

---

## 📋 Workflow Steps

### Step 1: Read Context

Before starting any iteration:

1. ✅ Read `vision.md` - understand architecture and design
2. ✅ Read `conventions.md` - understand coding rules
3. ✅ Read `tasklist.md` - identify current iteration
4. ✅ Read `idea.md` - understand requirements

### Step 2: Propose Solution

For each iteration, **BEFORE implementing**:

1. **Announce iteration**: State which iteration you're starting
2. **List tasks**: Show all tasks from that iteration
3. **Propose architecture**: Explain approach with code snippets
4. **Show structure**: Present key functions/structs/modules
5. **Ask for approval**: "Ready to implement? Any changes needed?"

#### Proposal Format:

```markdown
## 🔨 Iteration X: [Name]

### Tasks to Complete:
- [ ] Task 1
- [ ] Task 2
- [ ] Task 3

### Proposed Solution:

**File Structure:**
- path/to/file.rs

**Key Components:**

[Code snippet showing main structure]

**Approach:**
1. Step 1 explanation
2. Step 2 explanation
3. Step 3 explanation

### Testing Plan:
- Test case 1
- Test case 2

**Ready to implement? Any changes needed?**
```

### Step 3: Wait for Approval

⏸️ **STOP and wait for user response**

- If approved: Proceed to Step 4
- If changes requested: Update proposal, wait again
- If blocked: Document in `tasklist.md` progress table

### Step 4: Implement

Once approved:

1. **Create/modify files** according to proposal
2. **Write tests** for all main functions
3. **Follow conventions** from `conventions.md`
4. **Run quality checks**:
   - `cargo build` - must succeed
   - `cargo clippy` - fix all warnings
   - `cargo fmt` - format all code
   - `cargo test` - all tests must pass

### Step 5: Verify & Report

After implementation:

1. **Run the testing checklist** from the iteration
2. **Report results**:
   ```markdown
   ## ✅ Iteration X: Completed
   
   ### Implementation Summary:
   - Created: [files]
   - Modified: [files]
   - Tests added: [count]
   
   ### Test Results:
   - [x] Test 1: Passed
   - [x] Test 2: Passed
   - [ ] Test 3: Manual testing required
   
   ### Quality Checks:
   - [x] cargo build: Success
   - [x] cargo clippy: No warnings
   - [x] cargo fmt: Formatted
   - [x] cargo test: X/X tests passed
   
   ### Manual Testing Instructions:
   [If applicable, show commands to run for manual testing]
   
   **Ready for verification. Please test and confirm.**
   ```

### Step 6: Wait for Confirmation

⏸️ **STOP and wait for user to verify**

- User will test the implementation
- User will confirm: ✅ "Confirmed, proceed" or 🔄 "Fix needed"
- If fixes needed: Address issues, return to Step 5

### Step 7: Update Progress

After user confirmation:

1. **Update `tasklist.md`**:
   - Mark all tasks as completed [x]
   - Update progress table (status: ✅, completion: X%)
   - Add progress note with timestamp
2. **Commit changes** (if using git)

#### Update Example:

```markdown
## 📊 Progress Report

| Iteration | Description | Status | Completion |
|-----------|-------------|--------|------------|
| 0️⃣ | Project Setup | ✅ Completed | 100% |
| 1️⃣ | Common Library | 🔄 In Progress | 50% |

---

### [2025-11-10] - Iteration 0 Completed
- Duration: 30 minutes
- All workspace structure created
- Builds successfully
- Next: Iteration 1
```

### Step 8: Transition to Next Iteration

Before moving forward:

1. **Summarize completed iteration**
2. **Preview next iteration** (read from `tasklist.md`)
3. **Ask permission**: "Iteration X complete. Proceed to Iteration Y?"

⏸️ **STOP and wait for approval to continue**

### Step 9: Repeat

Go back to Step 2 for next iteration.

---

## 🚫 What NOT to Do

❌ **Don't** skip the proposal step  
❌ **Don't** implement without approval  
❌ **Don't** move to next iteration without confirmation  
❌ **Don't** deviate from `tasklist.md` plan  
❌ **Don't** skip testing  
❌ **Don't** ignore `conventions.md` rules  
❌ **Don't** leave broken code  
❌ **Don't** forget to update progress  

---

## 🔄 Iteration Template

Use this template for each iteration:

```markdown
---
## 🔨 Iteration X: [Name]
**Goal:** [Goal from tasklist.md]

### Proposed Solution:
[Code snippets and explanation]

**Ready to implement?**

---
[After approval]

### Implementation:
[Files created/modified]

---
[After implementation]

### ✅ Verification Results:
[Test results and quality checks]

**Ready for confirmation.**

---
[After confirmation]

### Updated tasklist.md ✅
Proceeding to Iteration Y?
---
```

---

## 📞 Communication Protocol

### When Proposing:
- Be specific with code examples
- Show file structure clearly
- Explain the "why" behind decisions
- Ask clear yes/no questions

### When Reporting:
- Show concrete test results
- Provide commands for manual testing
- Be honest about what works/doesn't work
- Include any warnings or issues encountered

### When Blocked:
- Clearly state the blocker
- Propose alternative solutions
- Ask specific questions
- Wait for guidance

---

## 🎯 Quality Gates

Before marking iteration as complete:

- [ ] All tasks in iteration checklist completed
- [ ] All tests in testing checklist passed
- [ ] Code follows `conventions.md`
- [ ] No compiler warnings
- [ ] No clippy warnings
- [ ] Code formatted with rustfmt
- [ ] User has confirmed functionality
- [ ] `tasklist.md` updated

**If any gate fails, iteration is not complete.**

---

## 📝 Example Full Iteration Flow

```
CODE ASSISTANT:
"🔨 Starting Iteration 0: Project Setup

I'll create the workspace structure with 3 crates.
[Shows proposed Cargo.toml and structure]

Ready to implement?"

USER: "Looks good, proceed"

CODE ASSISTANT:
[Implements files]
"✅ Implementation complete.

Created:
- Cargo.toml (workspace)
- quote_common/Cargo.toml
- quote_server/Cargo.toml
- quote_client/Cargo.toml
[etc.]

Quality checks:
- cargo build: ✅ Success

Ready for verification."

USER: "Confirmed, everything works"

CODE ASSISTANT:
"✅ Updated tasklist.md - Iteration 0 complete (100%)

Preview of Iteration 1: Common Library
- Define StockQuote struct
- Define QuoteError enum
[etc.]

Proceed to Iteration 1?"

USER: "Yes"

CODE ASSISTANT:
"🔨 Starting Iteration 1: Common Library
[Proposal for next iteration...]"
```

---

## 🎓 Remember

1. **Strict adherence** to plan
2. **Always propose** before implementing
3. **Always wait** for confirmation
4. **Always update** progress
5. **Always ask** before moving forward

**KISS Principle:** Keep communication clear, code simple, progress visible.

---

**Ready to start? Begin with Iteration 0 when approved.**
