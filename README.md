# Ark Server Manager: Ascended

**Latest Development Build**: [Dev Build](https://arkservermanager.s3.us-west-2.amazonaws.com/asma/release/latest-dev.zip)
> Development builds are not recommended for use by non-developers. They may be quite broken and could lead to data corruption.

**Latest Release**: *No builds available yet*

**Discord**: [Server Mangers](https://discord.gg/aY6erNcXef)

## Overview

This is the official repository for the Ark Server Manager: Ascended project - a tool used to configure and manage dedicated servers for the game Ark: Survival Ascended (tm) game by Studio Wildcard and Snail Games USA.  This tool is not an official Studio Wildcard tool, nor do its developers have any association with Studio Wildcard, Snail Games USA or their partners or affiliates. This product is provided free-of-charge to all users. This product is not packaged with Ark: Survival Ascended or any of its assets - these must be obtained through normal means (e.g. Steam, Epic, etc.), which may require an account with various services. The tool endeavors to make this process easy and automated for the end user.

## Goals and Rationale

This project aims to re-create and update the [Ark Server Manager](https://github.com/Bletch1971/ServerManagers/tree/source) originally written by HellsGuard, myself and Bletch over the course of many years, with contributions from many others in the form of testing and translations. Ark: Survival Ascended (tm) brings many changes that make the original ASM incompatible - simple things like data changes and more complex details like how to interact with the server as an administrator. While it is of course possible to extend the ASM to cover this functionality, Bletch has decided to stop supporting that project and I do not wish to spent any more time writing C# and WPF code on my personal time. Further, extending the existing code base would require maintaining compatibility with Ark: Survival Evolved (tm), or a fork and surgery to remove that which no longer applies, which means the code base stands in a confusing and indeterminate place for an arbitrary period of time. Add to that the fact that it is built on a very old version of C# and WPF and you can understand why doing that as a personal project might not seem fun.

## Technologies

This project uses [Rust](https://www.rust-lang.org/) and [slint](https://slint.dev/). This change from .NET brings some benefits and drawbacks.

Benefits:
* Rust makes it easy to produce robust and performant applications
* Easy integration with native libraries
* No extra runtime to install

Drawbacks:
* Can't re-use existing code
* UI libraries are not as rich and complete as for .NET
* Fewer developers who can develop and maintain the code

Personally, I just enjoy writing code in Rust compared to most other languages.

## Discussion and Participation

You can get involved in discussions about development on [Discord](https://discord.gg/aY6erNcXef) as well as submit Issues and PRs to this repository.