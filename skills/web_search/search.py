#!/usr/bin/env python3
# /// script
# requires-python = ">=3.10"
# dependencies = [
#     "ddgs>=6.0.0",
#     "httpx>=0.25.0",
#     "beautifulsoup4>=4.12.0",
#     "google-genai>=1.0.0",
# ]
# ///
"""
Web search tool using DuckDuckGo or Gemini with Google Search grounding.

Usage:
    uv run search.py "search query" [--max N] [--fetch] [--gemini]

Examples:
    uv run search.py "python async tutorial"
    uv run search.py "rust error handling" --max 5
    uv run search.py "Who won Euro 2024?" --gemini
    uv run search.py "latest AI news" --news

Requires GEMINI_API_KEY environment variable for --gemini.
"""

import sys
import json
import argparse
import os

def search_web(query: str, max_results: int = 5) -> list[dict]:
    """Search the web using DuckDuckGo."""
    from ddgs import DDGS

    results = []
    with DDGS() as ddgs:
        for r in ddgs.text(query, max_results=max_results):
            results.append({
                "title": r.get("title", ""),
                "url": r.get("href", ""),
                "snippet": r.get("body", "")
            })
    return results

def fetch_page(url: str, max_chars: int = 5000) -> str:
    """Fetch a web page and extract text content."""
    import httpx
    from bs4 import BeautifulSoup

    headers = {
        "User-Agent": "Mozilla/5.0 (compatible; eunice/1.0; +https://github.com/xeb/eunice)"
    }

    response = httpx.get(url, headers=headers, follow_redirects=True, timeout=10)
    response.raise_for_status()

    soup = BeautifulSoup(response.text, "html.parser")

    # Remove script and style elements
    for element in soup(["script", "style", "nav", "footer", "header"]):
        element.decompose()

    # Get text
    text = soup.get_text(separator="\n", strip=True)

    # Clean up whitespace
    lines = [line.strip() for line in text.splitlines() if line.strip()]
    text = "\n".join(lines)

    if len(text) > max_chars:
        text = text[:max_chars] + f"\n\n... (truncated, {len(text)} total chars)"

    return text

def search_news(query: str, max_results: int = 5) -> list[dict]:
    """Search news using DuckDuckGo."""
    from ddgs import DDGS

    results = []
    with DDGS() as ddgs:
        for r in ddgs.news(query, max_results=max_results):
            results.append({
                "title": r.get("title", ""),
                "url": r.get("url", ""),
                "snippet": r.get("body", ""),
                "date": r.get("date", ""),
                "source": r.get("source", "")
            })
    return results

def search_gemini(query: str, model: str = "gemini-3-flash-preview") -> dict:
    """Search using Gemini with Google Search grounding."""
    api_key = os.environ.get("GEMINI_API_KEY")
    if not api_key:
        return {"error": "GEMINI_API_KEY environment variable not set."}

    try:
        from google import genai
        from google.genai import types

        client = genai.Client(api_key=api_key)

        grounding_tool = types.Tool(
            google_search=types.GoogleSearch()
        )

        config = types.GenerateContentConfig(
            tools=[grounding_tool]
        )

        response = client.models.generate_content(
            model=model,
            contents=query,
            config=config,
        )

        result = {
            "query": query,
            "model": model,
            "response": response.text,
        }

        # Extract grounding metadata if available
        if hasattr(response, 'candidates') and response.candidates:
            candidate = response.candidates[0]
            if hasattr(candidate, 'grounding_metadata') and candidate.grounding_metadata:
                metadata = candidate.grounding_metadata
                if hasattr(metadata, 'search_entry_point') and metadata.search_entry_point:
                    result["search_entry_point"] = metadata.search_entry_point.rendered_content
                if hasattr(metadata, 'grounding_chunks') and metadata.grounding_chunks:
                    result["sources"] = [
                        {"uri": chunk.web.uri, "title": chunk.web.title}
                        for chunk in metadata.grounding_chunks
                        if hasattr(chunk, 'web') and chunk.web
                    ]

        return result
    except Exception as e:
        return {"error": f"Gemini search failed: {e}"}

def main():
    parser = argparse.ArgumentParser(description="Search the web using DuckDuckGo or Gemini")
    parser.add_argument("query", help="Search query")
    parser.add_argument("--max", type=int, default=5, help="Maximum results for DuckDuckGo (default: 5)")
    parser.add_argument("--fetch", action="store_true", help="Fetch first result's content (DuckDuckGo only)")
    parser.add_argument("--news", action="store_true", help="Search news (DuckDuckGo only)")
    parser.add_argument("--gemini", action="store_true", help="Use Gemini with Google Search grounding")
    parser.add_argument("--model", default="gemini-3-flash-preview",
                        help="Gemini model (default: gemini-3-flash-preview)")
    parser.add_argument("--json", action="store_true", help="Output as JSON")

    args = parser.parse_args()

    try:
        # Gemini search path
        if args.gemini:
            result = search_gemini(args.query, args.model)

            if "error" in result:
                print(f"Error: {result['error']}", file=sys.stderr)
                sys.exit(1)

            if args.json:
                print(json.dumps(result, indent=2))
            else:
                print(f"=== Gemini Search: {args.query} ===\n")
                print(result["response"])
                if "sources" in result and result["sources"]:
                    print("\n--- Sources ---")
                    for src in result["sources"]:
                        print(f"  - {src.get('title', 'Untitled')}")
                        print(f"    {src.get('uri', '')}")
            return

        # DuckDuckGo search path
        if args.news:
            results = search_news(args.query, args.max)
        else:
            results = search_web(args.query, args.max)

        if args.json:
            output = {"query": args.query, "results": results}
            if args.fetch and results:
                output["fetched_content"] = fetch_page(results[0]["url"])
            print(json.dumps(output, indent=2))
        else:
            print(f"=== Search Results for: {args.query} ===\n")

            if not results:
                print("No results found.")
                return

            for i, r in enumerate(results, 1):
                print(f"{i}. {r['title']}")
                print(f"   {r['url']}")
                if r.get('snippet'):
                    print(f"   {r['snippet'][:200]}...")
                if r.get('date'):
                    print(f"   Date: {r['date']} | Source: {r.get('source', 'N/A')}")
                print()

            if args.fetch and results:
                print(f"\n=== Content from: {results[0]['url']} ===\n")
                try:
                    content = fetch_page(results[0]["url"])
                    print(content)
                except Exception as e:
                    print(f"Failed to fetch page: {e}")

    except Exception as e:
        print(f"Error: {e}", file=sys.stderr)
        sys.exit(1)

if __name__ == "__main__":
    main()
