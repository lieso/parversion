pub struct Translation {
    pub basis_graph: BasisGraph,
    pub related_data: OutputData,
    pub translated_data: OutputData,
}

pub fn translate_file(
    file_name: String,
    options: Option<Options>,
    json_schema: String,
) -> Result<Translation, Errors> {
    log::trace!("In translate_file");
    log::debug!("file_name: {}", file_name);

    let mut text = String::new();

    let mut file = File::open(file_name).unwrap_or_else(|err| {
        eprintln!("Failed to open file: {}", err);
        process::exit(1);
    });

    file.read_to_string(&mut text).unwrap_or_else(|err| {
        eprintln!("Failed to read file: {}", err);
        process::exit(1);
    });

    translate_text(text, options, json_schema)
}

pub fn translate_text(
    text: String,
    options: Option<Options>,
    json_schema: String,
) -> Result<Translation, Errors> {
    log::trace!("In translate_text");

    let document = Document::from_string(text, options)?;

    translate_document(document, options, json_schema)
}

pub fn translate_document(
    document: Document
    options: Option<Options>,
    json_schema: String,
) -> Result<Translation, Errors> {
    log::trace!("In translate_document");

    let organization = organization::organize_document(document, options);

    translate_organization(organization, options, json_schema)
}

pub fn translate_organization(
    organization: Organization,
    options: Option<Options>,
    json_schema: String,
) -> Result<Translation, Errors> {
    log::trace!("In translate_document");

    let Organization {
        basis_graph,
        organized_data,
        related_data
    } = organization;

    let translated_data = basis_graph.translate(organized_data, json_schema);

    Translation {
        basis_graph,
        related_data,
        translated_data,
    }
}
