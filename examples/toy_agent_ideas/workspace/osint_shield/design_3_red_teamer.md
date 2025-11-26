# Design 3: The Social Engineer (Adversarial/Hybrid)

## Purpose
A "Red Team" agent that uses the data found (from Design 1 & 2) to simulate how an attacker could weaponize it. It generates realistic phishing lures or password cracking dictionaries based on your public data.

## Problem Domain
Users underestimate the risk of "harmless" data. Seeing a password dump is scary, but seeing a generated email that references your dog's name (found on Instagram) and your boss's name (found on LinkedIn) is visceral.

## Core Tools
- **memory**: Stores the gathered intelligence.
- **web**: Searches for "hooks" (events, conferences you attended, recent tweets).
- **filesystem**: Generates the "Attack Artifacts" (e.g., `spear_phishing_example.eml`, `custom_wordlist.txt`).
- **shell**: Can run local password strength testers (e.g., `cracklib`) against the generated wordlist.

## Main Loop
1.  **Reconnaissance**: Runs the logic from Design 2 to gather a profile.
2.  **Weaponization**:
    -   **Scenario 1 (Phishing)**: Uses specific details (e.g., "I saw your talk at X") to craft a high-credibility email template.
    -   **Scenario 2 (Auth)**: Generates a wordlist based on pet names, birth years, and usernames found online.
3.  **Simulation**:
    -   Does NOT send emails.
    -   Presents the user with the "Attack Package".
    -   "If you received this email, would you click it?"
4.  **Education**:
    -   Highlights exactly *which* data point made the attack possible (e.g., "This email works because you tweeted your location on Tuesday").

## Memory Architecture
- **Target Profile**: A centralized node representing the User, with attributes for "Vulnerabilities".

## Failure Modes
- **Safety**: Must ensure artifacts are never actually deployed (no emails sent). *Mitigation:* The agent has no email sending capability (SMTP), only file generation.
- **Offensiveness**: The generated content might be too aggressive or creepy. *Mitigation:* Strict system prompts for the generation step.

## Human Touchpoints
- **Opt-in**: User must explicitly start a "Campaign" against themselves.
- **Debrief**: Interactive session reviewing the findings.

## Key Insight
"Shock Therapy" for security. Most security tools are passive (shields); this one is active (sparring partner). It uses Generative AI to bridge the gap between "data exposure" and "exploitability".
