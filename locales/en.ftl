# English. The reference language: every other file has the same messages.
# Syntax: https://projectfluent.org/fluent/guide/

## Menu

app-tagline = Intrigue for the crown, against the computer
players = Players
player-count = { $count }  (you + { $pcs } PC)
strength = Computer strength
level-easy = Easy
level-medium = Medium
level-hard = Hard
start-game = Start game
speed-normal = Fast
speed-slow = Slow

## Players

player-you = You
player-pc = PC { $number }

## Cards

card-archer = Archer
card-archer-text = Eliminate the first or last card of the queue.
card-soldier = Soldier
card-soldier-text = Eliminate an adjacent card.
card-spy = Spy
card-spy-text = Steal 1 influence from a player with an adjacent card.
card-heir = Heir
card-heir-text = Gain 2 influence if no other Heir is revealed.
card-assassination = Assassination
card-assassination-text = Eliminate any card. Discard Assassination.
card-royal_decree = Royal Decree
card-royal_decree-text = Move a card anywhere in the queue. Discard Royal Decree.
card-lord = Lord
card-lord-text = Gain 1 influence, plus 1 per adjacent card of your family.
card-shapeshifter = Shapeshifter
card-shapeshifter-text = Copy the ability of an adjacent revealed character.
card-ambush = Ambush
card-ambush-text = Eliminated face down by an opponent: gain 4 and discard the attacker. Revealed by you: gain 1. Discard Ambush.
card-conspiracy = Conspiracy
card-conspiracy-text = Gain double the influence on it. Discard Conspiracy.

## Board

round-of = Round { $round } of { $rounds }
menu = Menu
influence = { $amount } influence
in-hand = { $count } in hand
queue-empty = The queue is empty.
face-down = face down
hidden = (hidden)
place = Place
here = here
move-here = Here
reveal = Reveal
keep-hidden = Keep hidden (+1)
pick-another = Pick another card
your-hand = Your hand
pick-to-see = Pick a card from your hand to see what it does.
copying = { $card } (copying { $copied })

## What to do now

game-is-over = The game is over.
thinking = { $player } is thinking...
pick-hand-card = Your turn: pick a card from your hand.
pick-place = Pick where to place your { $card }. It goes face down.
reveal-or-not = Your { $card } (marked): reveal it, or keep it hidden and add 1 influence to it?
decree-pick-card = Royal Decree: pick the card to move.
decree-pick-place = Royal Decree: pick the new place for the card.
pick-target = { $card }: pick a highlighted card.

## Story of the game. Each event is told as a sentence; the sentences about
## one card's turn make a paragraph. { $card } (the card that acts) and
## { $target } (the card acted upon) are drawn as a miniature and the card's
## name, or as log-hidden-card, so keep them in a form that needs no
## grammatical case. $player_you, $owner_you and $victim_you are "yes" when
## that player is the human. $lead is "yes" when the sentence opens its
## paragraph and "no" when it goes on about the same card.

story-actor = { $lead ->
    [yes] { $player_you ->
        [yes] Your
       *[other] { $player }'s
    } { $card }
   *[other] It
}
story-amount = { $amount } influence
story-round = Round { $round } begins.
story-placed-left = { $player } laid { $card } at the head of the queue.
story-placed-right = { $player } laid { $card } at the end of the queue.
story-placed-stack = { $player } laid { $card } on top of { $target }.
story-kept-hidden = { $player } kept { $card } hidden, and the stake on it grew to { $amount }.
story-revealed = { $player } turned over { $card }.
story-revealed-gain = { $player } turned over { $card } and collected { story-amount }.
story-gained = { story-actor } earned { story-amount }.
story-stole = { story-actor } reached past { $target } and stole 1 influence from { $victim_you ->
    [yes] you
   *[other] { $victim }
}.
story-copied = { story-actor } took on the guise of { $owner_you ->
    [yes] your
   *[other] { $owner }'s
} { $target }.
story-eliminated = { story-actor } struck down { $owner_you ->
    [yes] your
   *[other] { $owner }'s
} { $target }, and { $player_you ->
    [yes] you
   *[other] { $player }
} gained 1 influence.
story-eliminated-self = { story-actor } struck itself down, and { $player_you ->
    [yes] you
   *[other] { $player }
} gained 1 influence.
story-ambushed = { story-actor } attacked { $owner_you ->
    [yes] your
   *[other] { $owner }'s
} { $target } and fell into the trap: { $owner_you ->
    [yes] you
   *[other] { $owner }
} gained { story-amount }, { $player_you ->
    [yes] you
   *[other] { $player }
} gained 1.
story-ambushed-own = { story-actor } sprang { $player_you ->
    [yes] your
   *[other] { $player }'s
} own { $target }, which brought { story-amount }.
story-discarded = { story-actor } was then discarded.
story-moved = { story-actor } moved { $target } to position { $position } in the queue.
story-tally-final = When the round ended:
story-tally-now = So far:
story-tally-item = { $player }: { $total } ({ $change })

## Game log on the board and in the log window.

log-error = Error: { $error }
log-hidden-card = a face-down card
log-change = { $player } { $amount }
log-empty = Nothing has happened in this round yet.
log-open = Log
log-title = The story so far
log-round-tab = Round { $round }
log-close = Close
log-paused = The game waits while the story is open.

## Labels on cards while an action is shown

mark-copied = copied
mark-eliminated = eliminated
mark-ambush = Ambush! +{ $gained }
mark-discarded = discarded
mark-moves = moves

## Game over

you-win = You win!
you-share-win = You share the win!
winners = { $names } won
and = { " " }and{ " " }
tie-break = Ties are broken by the number of cards left in the queue.
back-to-menu = Back to menu
