# Security

## Reporting a problem

Please don't open a public issue for a security problem. Report it
privately through GitHub instead: open the repository's **Security** tab
and choose **Report a vulnerability**.

You'll get a reply within a week. Once a fix is released, the report is
published with credit to you, unless you'd rather stay anonymous.

## What counts

Sub20 reads and writes files on your computer and runs FFmpeg. Problems
worth reporting include:

- The window reaching files it wasn't given: through the asset protocol,
  `result_url`, `copy_file`, or the Linux preview server on 127.0.0.1.
- A crafted video or file name that makes Sub20 run something other than
  FFmpeg, or write outside the destination folder.
- An installer, update or bundled FFmpeg that isn't what was released.

Bugs in FFmpeg itself belong with the FFmpeg project.

## Supported versions

Only the latest release gets fixes.
