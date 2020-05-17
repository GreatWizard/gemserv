#!/usr/local/bin/perl


use SCGI;
use IO::Socket;
 
my $socket = IO::Socket::INET->new(Listen => 5, ReuseAddr => 1, LocalPort => 4001)
  	or die "cannot bind to port 4001: $!";
   
my $scgi = SCGI->new($socket, blocking => 1);
    
while (my $request = $scgi->accept) {
	$request->read_env;
	read $request->connection, my $body, $request->env->{CONTENT_LENGTH};
	print { $request->connection } "20\ttext/gemini\r\nperl\n";
	}
