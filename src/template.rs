struct Content {
    title: String,
    body: String,
    script: String,
}

impl Content {
    fn new(title: Option<String>, body: Option<String>, script: Option<String>) -> Self {
        let title = title.unwrap_or("Default".to_string());
        let body = body.unwrap_or("".to_string());
        let script = script.unwrap_or("".to_string());

        Self {
            title,
            body,
            script,
        }
    }
}

fn append(content: Content) -> String {
    let title = content.title;
    let body = content.body;
    let script = content.script;

    let template: String = format!(
        r#"
            <!DOCTYPE html>
            <html lang="en">
                <head>
                <meta charset="UTF-8">
                <meta name="viewport" content="width=device-width, initial-scale=0">
                <meta http-equiv="X-UA-Compatible" content="ie=edge">
                <link
                    rel="stylesheet"
                    href="https://cdn.jsdelivr.net/npm/@picocss/pico@2/css/pico.min.css"
                >
                <link rel="stylesheet" href="styles/index.css">
                <title>{}</title> 
                </head>
                <body>
                {}
                {}
                </body>
            </html>
        "#,
        title, body, script
    );

    template
}
