# Snapshot Browser API
Snapshot Browser is a system that allows files from read-only snapshots (e.g., btrfs snapshots) to be read conveniently.
It was designed for scenarios where snapshots are created frequently (e.g., every 15 minutes).
Each snapshot is contained in a folder with the data and time in the folder name.
Not all the snapshots are kept for a long time (i.e., a snapshot may be deleted as soon as a new snapshot is created).
Hence it is not possible to create stable links to the snapshots.
This system will provide easy access to the most recent snapshot.

## Architecture
The Snapshot Browser consists of two parts: an API and a front-end.
This repository contains the API.
The API is implemented as a REST web API.

## Functional Requirements

### Configuration
As an administrator I want to be able to configure the roots by specifying a snapshot path and a suffix for each root, so that I can easily define several roots.

### System information
As a developer I want an ```/info``` endpoint so that I can obtain basic system information as a JSON object containing the system name and version.

### Roots
As a developer I want a ```/roots``` endpoint so that I can obtain all snapshot roots configured in the system as a JSON object.

### Paths
As a developer I want a ```/roots/<root name>/paths/<path to file or directory>``` endpoint so that I can easily retrieve the directories and files of a snapshot.

### Paths - Directories
As a developer I want to obtain a JSON dictionary containing all sub-directories and files (including file size) if I call the "paths"-endpoint with a path pointing to a directory, so that I can traverse the directory tree.

### Paths - Files
As a developer I want to obtain the data of the file if I call the "paths"-endpoint with a path pointing to a file, so that I can work with the data of the file.

## Non-functional Requirements

### Programming language
The system must be implemented in Rust.

### Framework
The system must use the Rocket framework.

## License

This work is licensed under the MIT or Apache 2.0 license.

`SPDX-License-Identifier: MIT OR Apache-2.0`
