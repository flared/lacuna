#!/usr/bin/env -S uv run --script
#
# /// script
# dependencies = ["anthropic==0.84.0"]
# ///

import anthropic
from anthropic.types import TextBlock


def main() -> None:
    client = anthropic.Anthropic(
        base_url="https://lacuna.tailnet-name.ts.net/",
        api_key="-",
    )
    response = client.messages.create(
        model="claude-sonnet-4-5",
        max_tokens=4096,
        messages=[
            {
                "role": "user",
                "content": "Write haiku that mentions the word lacuna.",
            },
        ],
    )
    for block in response.content:
        if isinstance(block, TextBlock):
            print(block.text)
        else:
            raise TypeError(f"Unexpected block type: {block.type}")


if __name__ == "__main__":
    main()
