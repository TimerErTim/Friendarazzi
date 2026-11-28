//! Curated Quick-Start starter decks for low-friction prompt drafting.

use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DeckId(pub Uuid);

/// A themed collection of target-agnostic prompt templates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StarterDeck {
    pub id: DeckId,
    pub slug: String,
    pub name: String,
    pub description: String,
    pub prompts: Vec<String>,
}

/// Built-in Quick-Start decks seeded in storage and available in fakes.
pub fn builtin_starter_decks() -> Vec<StarterDeck> {
    vec![
        StarterDeck {
            id: DeckId(Uuid::from_u128(0x1000_0000_0000_0000_0000_0000_0000_0001)),
            slug: "night-out".into(),
            name: "Night Out".into(),
            description: "Bar-Abende, Club-Nächte und spontane Party-Momente.".into(),
            prompts: vec![
                "Bestellt noch eine Runde für alle".into(),
                "Schaut heimlich länger aufs Handy".into(),
                "Erklärt jemandem laut die Musik".into(),
                "Gestikuliert wild beim Erzählen".into(),
                "Hält zwei Getränke gleichzeitig".into(),
                "Sucht das eigene Glas".into(),
                "Singt lautstark beim Refrain mit".into(),
                "Macht ein Foto vom Essen oder Drink".into(),
                "Lacht über den eigenen Witz".into(),
                "Verliert kurz den Anschluss an die Gruppe".into(),
                "Bestellt einen Snack mitten in der Nacht".into(),
                "Prüft die Uhrzeit mit skeptischem Blick".into(),
                "Freundet sich mit jemandem am Nebentisch an".into(),
                "Korrigiert die Playlist oder wünscht sich einen Song".into(),
                "Lehnt sich dramatisch zurück".into(),
                "Stößt zum zweiten Mal mit derselben Person an".into(),
                "Zeigt jemandem ein altes Foto auf dem Handy".into(),
                "Gähnt heimlich hinter der Hand".into(),
                "Trägt Jacke oder Schal irgendwie schief".into(),
                "Versucht, eine Rechnung aufzuteilen".into(),
                "Tanzt kurz alleine im Stehen".into(),
                "Diskutiert ernsthaft über ein völlig absurdes Thema".into(),
                "Hält die Tür oder hält Platz frei".into(),
                "Fotografiert gerade selbst jemanden".into(),
                "Sieht komplett überrascht aus".into(),
                "Reicht ein Taschentuch oder Kaugummi weiter".into(),
                "Versucht über die Musik hinweg zu flüstern".into(),
                "Starrt auf die Getränkekarte wie auf ein Rätsel".into(),
                "Zählt am Tisch kurz alle Köpfe durch".into(),
                "Bindet Jacke oder Pullover um die Hüfte".into(),
                "Probiert heimlich vom Drink nebenan".into(),
                "Klatscht begeistert nach einem Song".into(),
                "Hält das Handy mit Blitz als Taschenlampe".into(),
                "Lehnt sich an Bar oder Wand für ein Gespräch".into(),
                "Ordnet Gläser oder Untersetzer ordentlich an".into(),
                "Macht eine ausladende Verabschiedungsgeste".into(),
            ],
        },
        StarterDeck {
            id: DeckId(Uuid::from_u128(0x1000_0000_0000_0000_0000_0000_0000_0002)),
            slug: "vacation-festival".into(),
            name: "Urlaub & Festival".into(),
            description: "Roadtrips, Strandtage, Camping und Festival-Chaos.".into(),
            prompts: vec![
                "Trägt Sonnenbrille trotz Schatten oder drinnen".into(),
                "Studiert konzentriert eine Karte oder den Zeitplan".into(),
                "Schleppt viel zu viel Gepäck auf einmal".into(),
                "Cremt sich oder andere mit Sonnencreme ein".into(),
                "Schläft an einem ungewöhnlichen Ort ein".into(),
                "Isst etwas im Gehen".into(),
                "Kämpft mit Wind, Zelt oder Regenschirm".into(),
                "Trägt eine wilde Kopfbedeckung".into(),
                "Fotografiert eine Landschaft oder Bühne".into(),
                "Sucht verzweifelt nach dem richtigen Weg".into(),
                "Balanciert Snacks und Getränke gleichzeitig".into(),
                "Sitzt auf dem Boden, obwohl Stühle frei sind".into(),
                "Klebt ein Pflaster oder sucht nach Verbandszeug".into(),
                "Füllt die Wasserflasche nach".into(),
                "Winkt jemandem aus der Ferne zu".into(),
                "Kramt minutenlang im Rucksack".into(),
                "Improvisiert einen Sonnenschutz".into(),
                "Zählt laut Geld oder Tickets".into(),
                "Trägt Sandalen mit Socken oder nasse Schuhe".into(),
                "Versucht Powerbank und Handy zu entwirren".into(),
                "Posiert vor einem Schild oder Denkmal".into(),
                "Testet die Wassertemperatur vorsichtig mit einem Fuß".into(),
                "Hat Sand, Gras oder Glitzer im Gesicht".into(),
                "Bestellt etwas in einer anderen Sprache mit Händen".into(),
                "Starrt verträumt in die Ferne".into(),
                "Schüttelt Sand aus Schuhen oder Handtuch".into(),
                "Kauft ein Souvenir oder einen Festival-Becher".into(),
                "Klebt das Armband oder Ticket stolz ans Handgelenk".into(),
                "Sucht nach Schatten für die kurze Pause".into(),
                "Faltet eine Decke oder Jacke als Kissen".into(),
                "Reicht Snacks durch die Sitzreihe".into(),
                "Lauscht mit geschlossenen Augen der Musik".into(),
                "Hält Ausschau nach den anderen auf dem Gelände".into(),
                "Bindet die Schuhe auf einem Koffer oder Stein".into(),
                "Streckt die Füße in die Sonne oder ins Gras".into(),
                "Lacht über ein Missgeschick beim Aufbau".into(),
            ],
        },
        StarterDeck {
            id: DeckId(Uuid::from_u128(0x1000_0000_0000_0000_0000_0000_0000_0003)),
            slug: "roommates".into(),
            name: "WG & Zuhause".into(),
            description: "Küche, Couch, Spieleabend und alltägliche WG-Gewohnheiten.".into(),
            prompts: vec![
                "Kocht oder rührt konzentriert im Topf".into(),
                "Steht barfuß oder in dicken Socken in der Küche".into(),
                "Trinkt direkt aus einer riesigen Tasse".into(),
                "Starrt fragend in den offenen Kühlschrank".into(),
                "Wickelt sich komplett in eine Decke ein".into(),
                "Räumt heimlich Geschirr weg".into(),
                "Liegt quer über Sofa oder Sessel".into(),
                "Nascht direkt aus der Packung".into(),
                "Liest eine Verpackungsrückseite extrem genau".into(),
                "Gießt oder begutachtet eine Zimmerpflanze".into(),
                "Trägt zwei unterschiedliche Socken oder Homewear".into(),
                "Balanciert Teller und Tasse ins Wohnzimmer".into(),
                "Diskutiert Spielregeln leidenschaftlich".into(),
                "Schneidet Zwiebeln oder Gemüse hochkonzentriert".into(),
                "Sucht nach einer Schere oder Klebeband".into(),
                "Kocht Wasser für Tee oder Kaffee".into(),
                "Hat Kopfhörer nur auf einem Ohr".into(),
                "Sortiert Snacks auf dem Tisch".into(),
                "Schläft halb auf der Couch ein".into(),
                "Probiert heimlich aus dem Topf".into(),
                "Sucht die verlegte Brille oder das Handy".into(),
                "Öffnet ein Paket mit viel zu großem Werkzeug".into(),
                "Hält Haustier oder Kuschelkissen im Arm".into(),
                "Schaut aus dem Fenster wie in einem Musikvideo".into(),
                "Macht den Spülmaschinen-Tetris-Meister".into(),
                "Sitzt im Schneidersitz auf dem Küchenstuhl".into(),
                "Hält eine Fernbedienung wie ein Mikrofon".into(),
                "Sucht die passende Tupperdose zum Deckel".into(),
                "Summt eine Melodie beim Kochen oder Putzen".into(),
                "Stellt eine Kerze an oder dimmt das Licht".into(),
                "Blättert in einer alten Anleitung oder einem Magazin".into(),
                "Reibt sich verschlafen die Augen".into(),
                "Faltet Wäsche oder Socken mitten im Raum".into(),
                "Testet vorsichtig, ob der Tee schon trinkbar ist".into(),
                "Hält ein Brettspiel-Teil nachdenklich in der Hand".into(),
                "Lacht über ein Video auf dem eigenen Bildschirm".into(),
            ],
        },
        StarterDeck {
            id: DeckId(Uuid::from_u128(0x1000_0000_0000_0000_0000_0000_0000_0004)),
            slug: "office-uni".into(),
            name: "Büro & Uni".into(),
            description: "Kaffeemaschine, Lernmarathons, Mensa und Meetings.".into(),
            prompts: vec![
                "Holt den zweiten oder dritten Kaffee".into(),
                "Starrt hochkonzentriert auf den Laptop".into(),
                "Schreibt mit drei verschiedenen Textmarkern".into(),
                "Trägt mehr Notizbücher als nötig".into(),
                "Gähnt mitten im Gespräch".into(),
                "Klebt Post-its an Bildschirm oder Heft".into(),
                "Balanciert Mensa-Tablett oder Brotdose".into(),
                "Flucht leise über den Drucker oder das WLAN".into(),
                "Sitzt extrem schief auf dem Bürostuhl".into(),
                "Trägt Kopfhörer als Nicht-stören-Schild".into(),
                "Reibt sich nachdenklich die Stirn".into(),
                "Sucht ein Ladekabel oder eine Steckdose".into(),
                "Zeichnet Kästchen oder Pfeile ans Whiteboard".into(),
                "Isst einen Pausensnack direkt über der Tastatur".into(),
                "Prüft die Mensa- oder Mittagsoptionen".into(),
                "Erklärt etwas leidenschaftlich am Bildschirm".into(),
                "Sammelt lose Blätter zusammen".into(),
                "Lacht laut im stillen Lernraum oder Flur".into(),
                "Hält eine Wasserflasche wie eine Trophäe".into(),
                "Hat den Rucksack noch halb auf einer Schulter".into(),
                "Streckt sich dramatisch nach langer Sitzphase".into(),
                "Nickt zustimmend, obwohl gerade kaum zugehört wurde".into(),
                "Wartet ungeduldig vor der Kaffeemaschine".into(),
                "Vergleicht zwei Ausgaben oder Folien nebeneinander".into(),
                "Schaut panisch auf die Uhr".into(),
                "Tippt hochmotiviert mit hörbarem Tastatur-Tempo".into(),
                "Balanciert drei Dinge gleichzeitig zum Schreibtisch".into(),
                "Macht ein heimliches Stretch-Programm am Platz".into(),
                "Schaut kurz ratlos auf ein weißes Blatt".into(),
                "Tauscht einen Stift oder Marker aus der Runde".into(),
                "Trinkt aus einer völlig zerbeulten Trinkflasche".into(),
                "Hebt skeptisch eine Augenbraue im Gespräch".into(),
                "Sortiert Stifte oder Kabel in Reih und Glied".into(),
                "Schreibt etwas hektisch auf eine Serviette oder Hand".into(),
                "Blickt sehnsüchtig Richtung Kaffeeküche oder Ausgang".into(),
                "Klopft leise mit dem Stift den Takt mit".into(),
            ],
        },
    ]
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;
    use crate::domain::game::PROMPT_MAX_CHARS;

    #[test]
    fn builtin_starter_decks_are_target_agnostic_and_unique() {
        let decks = builtin_starter_decks();
        assert_eq!(decks.len(), 4);

        for deck in decks {
            assert_eq!(
                deck.prompts.len(),
                36,
                "deck {} should have 36 prompts",
                deck.slug
            );
            let mut seen = HashSet::new();
            for prompt in &deck.prompts {
                let trimmed = prompt.trim();
                assert!(!trimmed.is_empty(), "empty prompt in deck {}", deck.slug);
                assert!(
                    trimmed.chars().count() <= PROMPT_MAX_CHARS,
                    "prompt exceeds {PROMPT_MAX_CHARS} chars in deck {}: {trimmed}",
                    deck.slug
                );
                assert!(
                    !trimmed.contains("{name}"),
                    "prompt still contains {{name}} in deck {}: {trimmed}",
                    deck.slug
                );
                let first = trimmed.chars().next().expect("non-empty");
                assert!(
                    first.is_uppercase(),
                    "prompt should start uppercase in deck {}: {trimmed}",
                    deck.slug
                );
                assert!(
                    seen.insert(trimmed.to_lowercase()),
                    "duplicate prompt in deck {}: {trimmed}",
                    deck.slug
                );
            }
        }
    }
}
