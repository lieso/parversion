impl Node {
    pub fn log_tree(&self, title: &str) {

        //let xml = tree_to_xml(tree.clone());
        //let xml_file_name = format!("tree_{}.xml", tree.ancestry_hash());

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open("./debug/trees")
            .expect("Could not open file");

        let divider = std::iter::repeat("*").take(100).collect::<String>();
        let text = format!(
            "\n\n{} {}\n",
            divider,
            title
        );

        writeln!(file, "{}", text).expect("Could not write to file");

        let mut node_count = 0;

        bfs(self.clone().into(), &mut |node: &Rc<Node>| {
            node_count = node_count + 1;

            let divider = std::iter::repeat("-").take(50).collect::<String>();
            let text = format!(
                "\nID: {}\nHASH: {}\nXML: {}\nSUBTREE HASH: {}\nANCESTOR HASH: {}\nCOMPLEX TYPE NAME: {:?}\n",
                node.id,
                node.hash,
                node.xml,
                node.subtree_hash(),
                node.ancestry_hash(),
                node.complex_type_name
            );

            let mut node_data_text = String::from("");

            for d in node.data.borrow().iter() {
                node_data_text = node_data_text + format!(r##"
                    name: {},
                    value: {:?}
                "##, d.name, d.value).as_str();
            }

            let text = format!("\n{}{}{}{}\n", divider, text, node_data_text, divider);

            writeln!(file, "{}", text).expect("Could not write to file");
        });

        writeln!(file, "node count: {}", node_count).expect("Could not write to file");
    }
}
