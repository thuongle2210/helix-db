N::File999 {
    INDEX name: String,
    INDEX age: I32,
    INDEX count: F32,
}

E::EFile999 {
    From: File9,
    To: File9,
}


QUERY file999(other_id: ID, id: ID) =>
    path1 <- N<File999>(id)::ShortestPath<File999>::To(other_id)
    path2 <- N<File999>(id)::ShortestPath<File999>::From(other_id)
    RETURN path1, path2