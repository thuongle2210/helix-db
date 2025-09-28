N::File9 {
    INDEX name: String,
    INDEX age: I32,
    INDEX count: F32,
}

E::EFile9 {
    From: File9,
    To: File9,
}


QUERY file9(other_id: ID, id: ID) =>
    path1 <- N<File9>(id)::CyclePath<File9>
    path2 <- N<File9>(id)::CyclePath<File9>
    RETURN path1, path2