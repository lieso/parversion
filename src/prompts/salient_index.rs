pub static SALIENT_INDEX: &str = r##"
Given an input of HTML content, determine approximately where the salient content begins. The salient content refers to the main body of text that is meant for the user to read, such as articles, blog posts, or product descriptions. This excludes navigational elements, headers, footers, forms, and other similar page components.

If salient content is identified, provide the response in a JSON format with a key called "content_index" representing the starting index of the content. If salient content cannot be found, provide a JSON response with a "status" key with the value "failure" and a "message" key indicating that no salient content could be identified. Do not include introduction or final summary in response.

For example:

Input HTML:
<!DOCTYPE html>
<html>
<head>
    <title>Example Page</title>
</head>
<body>
    <nav>...</nav>
    <header>...</header>
    <main>
        <article>
            Here is the salient content that we want to identify.
        </article>
    </main>
    <footer>...</footer>
</body>
</html>

Desired JSON Output if salient content is found:
{
  "status": "success",
  "content_index": 123
}

Desired JSON Output if no salient content is found:
{
  "status": "failure",
  "message": "Salient content could not be determined."
}

Using the above guidance, analyze the following HTML content and generate the appropriate JSON response with the requested information.
"##;
