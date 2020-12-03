use crate::peer::Peer;

pub fn print_peers(peers: &Vec<Peer>) {
    let new_peers = peers.iter()
        .map(|peer| &peer.address()[(peer.address().len()-2)..])
        .map(|digits| digits.parse::<usize>().unwrap())
        .map(|index| NODES[index])
        .collect::<Vec<char>>();
    log::info!("my new peers: {:?}", new_peers);
}

pub const NODES: [char; 33] = [
    '🦋',

    '🌶',
    '🌽',
    '🥔',
    '🥦',
    '🧄',
    '🥨',
    '🍔',
    '🌶',

    '🥕',
    '🍆',
    '🧅',
    '🥒',
    '🥜',
    '🥐',
    '🥩',
    '🍠',

    '🍡',
    '🦀',
    '🦪',
    '🍦',
    '🍰',
    '🧁',
    '🍯',
    '🍷',

    '🍺',
    '🥛',
    '🍫',
    '🦑',
    '🥟',
    '🍚',
    '🍱',
    '🍕',
];