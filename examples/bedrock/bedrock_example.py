#!/usr/bin/env -S uv run --script
#
# /// script
# dependencies = ["boto3==1.42.63"]
# ///

import boto3
import json


def main() -> None:
    client = boto3.client(
        service_name="bedrock-runtime",
        region_name="us-east-1",
        endpoint_url="https://lacuna.tailnet-name.ts.net/bedrock/",
    )
    model_id = "us.anthropic.claude-opus-4-5-20251101-v1:0"
    payload = {
        "anthropic_version": "bedrock-2023-05-31",
        "max_tokens": 512,
        "messages": [
            {
                "role": "user",
                "content": "Write haiku that mentions the word lacuna.",
            }
        ],
    }
    response = client.invoke_model(
        modelId=model_id,
        body=json.dumps(payload),
    )
    result = json.loads(response.get("body").read())
    print(result.get("content")[0].get("text"))


if __name__ == "__main__":
    main()
